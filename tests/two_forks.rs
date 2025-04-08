    mod mock;

    use std::task::Poll;

    use futures::{SinkExt, executor::block_on};
    use mock::ForkAsyncMockSetup;

    #[test]
    fn nothing() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);

    
    }

    #[test]
    fn one_pending_send_one() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
       

        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));

        assert_eq!(fork2.next_a(), Poll::Pending);
    }

    #[test]
    fn both_pending_send_one() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 2>::new();

        let [mut fork1, mut fork2] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork2.next_a(), Poll::Pending);
        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        
         assert_eq!(fork2.next_a(), Poll::Ready(Some(())));
    }
    