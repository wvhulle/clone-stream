    mod mock;

    use std::task::Poll;

    use futures::{SinkExt, executor::block_on};
    use mock::ForkAsyncMockSetup;

    #[test]
    fn first_waker_unaffected() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);

        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
    }

    #[test]
    fn second_waker_also_consumed() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork1.next_b(), Poll::Pending);
        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_a(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_b(), Poll::Pending);

    }
    
        #[test]
    fn first_waker_also_consumed() {
        let ForkAsyncMockSetup {
            mut sender, forks, ..
        } = ForkAsyncMockSetup::<(), 1>::new();

        let [mut fork1] = forks;

        assert_eq!(fork1.next_a(), Poll::Pending);
        assert_eq!(fork1.next_b(), Poll::Pending);
        block_on(async {
            let _ = sender.send(()).await;
        });
        assert_eq!(fork1.next_b(), Poll::Ready(Some(())));
        assert_eq!(fork1.next_a(), Poll::Pending);

    }
