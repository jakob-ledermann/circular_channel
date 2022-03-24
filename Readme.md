# Motivation

To introduce rust into an existing project I need a reliable way to send data from a single realtime producer to a non-realtime consumer.
The existing channel implementations in std and crossbeam were not sufficient as the project needs to have a bounded channel and the sender may never block.

Imagine one thread sampling an ADC and sending the samples over the network to optional clients for advanced display.
As the sample is processed and the results of this processing is also provided with digital outputs in realtime, the sender must have an upper bound of the execution time.

This implementation is still only a private project to explore the technological feasibility of those requirements.
I also use it to further my understanding and knowledge of tools to implement safe multithreading abstractions.

The implementation is not targeted for use with multiple producers or multiple consumers.
