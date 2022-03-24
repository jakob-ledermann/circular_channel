use std::sync::Arc;

use circular_buffer::CircularBuffer;

/// Represents the sender of this channel
///
/// # Examples
///
/// ```
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// tx.send(1);
/// assert_eq!(rx.recv(), Some(1));
/// ```
///
/// ```compile_fail,E0382
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(move || {
///     tx.send(1);
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(move || {
///     tx.send(1);
/// });
///
/// ```
pub struct Sender<T> {
    inner: Arc<CircularBuffer<T>>,
}

/// # SAFETY:
/// - the innner channel is safe to use by one sender thread and one receiver thread at a time.
/// - the element needs to be send otherwise the following snippet would be a datarace
/// ```compile_fail
/// // this code snippet was compiling when the trait bound T: Send was missing
/// // and demonstrates a data race in safe rust using this channel.
/// // credit goes to discord user 5225225#6437 who pointed it out to me on the Rust Programming Language Community Discord Server.
/// use circular_channel::circular_channel;
/// use std::rc::Rc;
/// let x = Rc::new(());
/// let (send, recv) = circular_channel(1);
///              
/// send.send(x.clone());
///                                        
/// let t = std::thread::spawn(||{
///    let r = recv;
///    r.recv();
/// });
/// drop(x);
/// t.join().unwrap();
/// ```
unsafe impl<T: Send> Send for Sender<T> {}

/// Represents the receiver for this channel
/// this receiver can not be used by multiple threads at the same time.
///
/// # Examples
/// ```
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// tx.send(1);
/// assert_eq!(rx.recv(), Some(1));
/// ```
///
/// ```compile_fail,E0382
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(move || {
///     rx.recv();
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(move || {
///     rx.recv();
/// });
/// ```
pub struct Receiver<T> {
    inner: Arc<CircularBuffer<T>>,
}

/// # SAFETY:
/// - the innner channel is safe to use by one sender thread and one receiver thread at a time.
/// - the element needs to be send otherwise the following snippet would be a datarace
/// ```compile_fail
/// use circular_channel::circular_channel;
/// use std::rc::Rc;
/// let x = Rc::new(());
/// let (send, recv) = circular_channel(1);
///              
/// send.send(x.clone());
///                                        
/// let t = std::thread::spawn(||{
///    let r = recv;
///    r.recv();
/// });
/// drop(x);
/// t.join().unwrap();
/// ```
unsafe impl<T: Send> Send for Receiver<T> {}

/// this creates a channel for passing messages between threads
/// this channel differs from the commonly used and seen channel in std::mpsc and the crate crossbeam in the following properties:
/// - it only allocates once on construction
/// - the number of pending messages is always limited
/// - if the sender wants to send a new message and the limit of pending messages is reached the oldest not yet processed message is dropped from the channel.
///   the above mentioned channel implementations do not provide a way of dropping the oldest pending message they only offer:
///   - block the sender
///   - fail to send and let caller handle the fallout
///     arguably it might be possible to use a crossbeam bounded multi consumer channel in a way to achive this behaviour
///     by providing the producing thread with the sender and a receiver and call try_recv if try_send fails due to having reached the bound
///
/// # Examples
///
/// ```
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
/// tx.send(1);
/// assert_eq!(rx.recv(), Some(1));
/// ```
///
/// the sender can only be moved between threads and there can never be two threads sending to the same channel
///
/// ```compile_fail,E0382
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(move || {
///     tx.send(1);
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(move || {
///     tx.send(1);
/// });
///
/// ```
/// ```compile_fail, E0277
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(|| {
///     tx.send(1);
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(|| {
///     tx.send(1);
/// });
///
/// ```
/// the same goes for the receiver:
///
/// ```compile_fail, E0382
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(move || {
///     rx.recv();
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(move || {
///     rx.recv();
/// });
///
/// ```
///
/// ```compile_fail,E0277
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// std::thread::spawn(|| {
///     rx.recv();
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(|| {
///     rx.recv();
/// });
///
/// ```
///
/// ```compile_fail,E0277
/// use std::sync::Arc;
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// let sender = Arc::new(tx);
/// let sender_2 = sender.clone();
/// std::thread::spawn(|| {
///     sender_2.send(1);
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(|| {
///     sender_2.send(1);
/// });
///
/// ```
///
/// ```compile_fail,E0277
/// use std::sync::Arc;
/// use circular_channel::circular_channel;
/// let (tx, rx) = circular_channel::<i32>(5);
///
/// let sender = Arc::new(rx);
/// let sender_2 = sender.clone();
/// std::thread::spawn(|| {
///     sender_2.recv();
/// });
///
/// // this is may never be allowed by the compiler
/// std::thread::spawn(|| {
///     sender_2.recv();
/// });
///
/// ```
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

#[cfg(test)]
mod tests {}

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

            assert!(write_index < self.capacity);
            // SAFETY:
            // - buffer points to the start of the memory region of a Vec<MaybeUninit<T>> with capacity self.capacity
            //   so this call is equivalent as a vec::with_capacity(self.capacity).as_mut_ptr().offset(write_index) and combined with the assert thus safe.
            let offset = unsafe { self.buffer.offset(write_index.try_into().unwrap()) };

            if read_index == write_index && len > 0 {
                // The receiver is slower than the sender and we want to throw the oldest unprocessed element away
                let new_read_index = self.increment_index(read_index);
                added_count = 0;

                match self.read_index.compare_exchange(
                    read_index,
                    new_read_index,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => {
                        // TODO: we are responsible for running the destructor on the element, that was at this index, as we are going to overwrite it
                        // SAFETY:
                        // - offset points to a valid instance of MaybeUninit<T>
                        // - as we just advanced the read_index this element would have been the next element retrieved, therefore it was previously written to and is a valid instance of T
                        unsafe {
                            offset.read().assume_init();
                        }
                    }
                    Err(_) => {
                        // this result is not relevant as the receiver retrieved this element.
                    }
                }
            }

            let new_write_index = self.increment_index(write_index);

            self.write_index.store(new_write_index, Ordering::Release);

            let container = MaybeUninit::new(element);

            // SAFETY:
            // - buffer is an aligned pointer to allocated memory for capacity elements of type MaybeUninit<T>
            // - write_index always holds: 0 <= write_index < capacity
            // - the resulting pointer is aligned and valid so can be used by the write
            unsafe {
                offset.write(container);
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

        #[test]
        fn buffer_returns_inserted_elements_in_order_overwrites_oldest_on_exceeding_capacity_drop_is_called_on_all_elements(
        ) {
            let buffer = CircularBuffer::<Box<i32>>::new(5);
            buffer.store(Box::new(1));
            buffer.store(Box::new(2));
            buffer.store(Box::new(3));
            buffer.store(Box::new(4));
            buffer.store(Box::new(5));
            buffer.store(Box::new(6));

            assert_eq!(buffer.retrieve(), Some(Box::new(2)));
            assert_eq!(buffer.retrieve(), Some(Box::new(3)));
            assert_eq!(buffer.retrieve(), Some(Box::new(4)));
            assert_eq!(buffer.retrieve(), Some(Box::new(5)));
            assert_eq!(buffer.retrieve(), Some(Box::new(6)));
            assert_eq!(buffer.retrieve(), None);
        }
    }
}
