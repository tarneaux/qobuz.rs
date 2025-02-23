use std::mem;
use tokio::sync::{mpsc, oneshot, watch};

pub(super) fn channel<T: Send + Sync + 'static>(
) -> (mpsc::Sender<T>, oneshot::Receiver<watch::Receiver<T>>) {
    // Capacity is 1 since we want to forward messages immediately when receiving them
    // + we can guarantee that the receiver won't block for too long since it's handled in this
    // module.
    let (in_tx, in_rx) = mpsc::channel(1);
    let (out_tx, out_rx) = oneshot::channel();
    tokio::spawn(async move { transmit_while_open(in_rx, out_tx).await });
    (in_tx, out_rx)
}

pub async fn transmit_while_open<T: Sync>(
    mut in_rx: mpsc::Receiver<T>,
    out_tx: oneshot::Sender<watch::Receiver<T>>,
) {
    let mut transmitter = Transmitter::NotInitialized(out_tx);
    while let Some(msg) = in_rx.recv().await {
        transmitter.send_replace(msg);
    }
}

pub(super) type DelayedWatchReceiver<T> = oneshot::Receiver<watch::Receiver<T>>;

enum Transmitter<T> {
    NotInitialized(oneshot::Sender<watch::Receiver<T>>),
    Initialized(watch::Sender<T>),
}

impl<T> Transmitter<T> {
    fn send_replace(&mut self, msg: T) {
        // This is safe because we are reassigning to self just after having zeroed it.
        *self = match mem::replace(self, unsafe { mem::zeroed() }) {
            Self::NotInitialized(sender) => {
                let (tx, rx) = watch::channel(msg);
                let _ = sender.send(rx);
                Self::Initialized(tx)
            }
            Self::Initialized(tx) => {
                tx.send_replace(msg);
                Self::Initialized(tx)
            }
        }
    }
}
