use std::sync::Arc;

use circular_buffer::CircularBuffer;

pub struct Sender<T> {
    inner: Arc<CircularBuffer<T>>,
}

// SAFETY: the innner channel is safe to use by one sender thread and one receiver thread
unsafe impl<T> Send for Sender<T> {}

pub struct Receiver<T> {
    inner: Arc<CircularBuffer<T>>,
}

// SAFETY: the innner channel is safe to use by one sender thread and one receiver thread
unsafe impl<T> Send for Receiver<T> {}

pub fn circular_channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let channel = CircularBuffer::new(capacity);
    let channel = Arc::new(channel);

    let sender = Sender {
        inner: channel.clone(),
    };
    let receiver = Receiver { inner: channel };
    (sender, receiver)
}

impl<T> Sender<T> {
    pub fn send(&self, element: T) {
        self.inner.store(element)
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Option<T> {
        self.inner.retrieve()
    }
}

mod circular_buffer {
    use std::{
        mem::MaybeUninit,
        sync::atomic::{AtomicUsize, Ordering},
    };

    pub(crate) struct CircularBuffer<T> {
        buffer: *mut MaybeUninit<T>,
        capacity: usize,
        len: AtomicUsize,
        read_index: AtomicUsize,
        write_index: AtomicUsize,
    }

    impl<T> Drop for CircularBuffer<T> {
        fn drop(&mut self) {
            // Safety:
            // - the buffer was created using a Vec with the original capacity so the pointer self.buffer is valid for self.capacity
            // - self.len contains the current number of elements in the buffer
            let element = unsafe {
                Vec::from_raw_parts(
                    self.buffer,
                    self.len.load(std::sync::atomic::Ordering::SeqCst),
                    self.capacity,
                )
            };
            core::mem::drop(element);
        }
    }

    impl<T: Sized> CircularBuffer<T> {
        pub fn new(capacity: usize) -> Self {
            let mut storage = Vec::with_capacity(capacity);
            let buffer = storage.as_mut_ptr();
            storage.leak();
            CircularBuffer {
                buffer: buffer,
                capacity: capacity,
                len: AtomicUsize::new(0),
                read_index: AtomicUsize::new(0),
                write_index: AtomicUsize::new(0),
            }
        }

        pub fn store(&self, element: T) {
            let read_index = self.read_index.load(Ordering::SeqCst);
            let write_index = self.write_index.load(Ordering::Acquire);
            let len = self.len.load(Ordering::Acquire);
            let mut added_count = 1;

            if read_index == write_index && len > 0 {
                // The receiver is slower than the sender and we want to throw the oldest unprocessed element away
                let new_read_index = self.increment_index(read_index);
                added_count = 0;

                // The result is irrelevant
                let _ = self.read_index.compare_exchange(
                    read_index,
                    new_read_index,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }

            let new_write_index = self.increment_index(write_index);

            self.write_index.store(new_write_index, Ordering::Release);

            let container = MaybeUninit::new(element);

            // SAFETY:
            // -
            unsafe {
                self.buffer
                    .offset(write_index.try_into().unwrap())
                    .write(container);
            };

            self.len.fetch_add(added_count, Ordering::Release);
        }

        pub fn retrieve(&self) -> Option<T> {
            for _ in 0..self.capacity {
                let read_index = self.read_index.load(Ordering::SeqCst);
                let len = self.len.load(Ordering::SeqCst);

                if len == 0 {
                    // the buffer is empty
                    return None;
                } else {
                    // SAFETY:
                    // - the buffer is valid memory with self.capacity elements of type MaybeUninit<T>
                    // - read_index is always < self.capacity
                    // - it might be that the sender is writing to the same location at the moment, which is why the elements are wrapped in MaybeUninit<T>
                    let element: MaybeUninit<T> = unsafe {
                        self.buffer
                            .offset(read_index.try_into().unwrap())
                            .replace(MaybeUninit::<T>::zeroed())
                    };
                    let new_read_index = self.increment_index(read_index);
                    match self.read_index.compare_exchange(
                        read_index,
                        new_read_index,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => {
                            // SAFETY:
                            // - element was written to as the length is bigger than 0
                            // - element is still valid, as we were able to increment the read_index after moving the value out of the storage location.
                            //   This might have failed if the sender overwrote the element we are trying to read
                            let element = unsafe { element.assume_init() };
                            self.len.fetch_sub(1, Ordering::SeqCst);
                            return Some(element);
                        }
                        Err(_) => {
                            // This is the case, if the sender was overwriting our data before or while we were reading it.
                            continue;
                        }
                    }
                }
            }

            panic!("The receiver is too slow and unable to retrieve any elements before they are overwritten.")
        }

        fn increment_index(&self, index: usize) -> usize {
            match index + 1 {
                x if x >= self.capacity => x % self.capacity,
                x => x,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::CircularBuffer;

        #[test]
        fn increment_index_wraps_on_capacity() {
            let buffer = CircularBuffer::<i32>::new(5);
            assert_eq!(buffer.increment_index(0), 1);
            assert_eq!(buffer.increment_index(1), 2);
            assert_eq!(buffer.increment_index(2), 3);
            assert_eq!(buffer.increment_index(3), 4);
            assert_eq!(buffer.increment_index(4), 0);

            let buffer = CircularBuffer::<i32>::new(3);
            assert_eq!(buffer.increment_index(0), 1);
            assert_eq!(buffer.increment_index(1), 2);
            assert_eq!(buffer.increment_index(2), 0);
            assert_eq!(buffer.increment_index(3), 1);
            assert_eq!(buffer.increment_index(4), 2);
        }

        #[test]
        fn empty_buffer_returns_none() {
            let buffer = CircularBuffer::<i32>::new(5);
            assert_eq!(buffer.retrieve(), None);
        }

        #[test]
        fn buffer_returns_inserted_elements_in_order_less_than_capacity() {
            let buffer = CircularBuffer::<i32>::new(5);
            buffer.store(1);
            buffer.store(2);
            buffer.store(3);
            buffer.store(4);

            assert_eq!(buffer.retrieve(), Some(1));
            assert_eq!(buffer.retrieve(), Some(2));
            assert_eq!(buffer.retrieve(), Some(3));
            assert_eq!(buffer.retrieve(), Some(4));
            assert_eq!(buffer.retrieve(), None);
        }

        #[test]
        fn buffer_returns_inserted_elements_in_order_overwrites_oldest_on_exceeding_capacity() {
            let buffer = CircularBuffer::<i32>::new(5);
            buffer.store(1);
            buffer.store(2);
            buffer.store(3);
            buffer.store(4);
            buffer.store(5);
            buffer.store(6);

            assert_eq!(buffer.retrieve(), Some(2));
            assert_eq!(buffer.retrieve(), Some(3));
            assert_eq!(buffer.retrieve(), Some(4));
            assert_eq!(buffer.retrieve(), Some(5));
            assert_eq!(buffer.retrieve(), Some(6));
            assert_eq!(buffer.retrieve(), None);
        }
    }
}
