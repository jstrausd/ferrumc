use super::*;

fn policy(groups: usize, bytes: usize, prepare: usize) -> ItemPublicationLimits {
    ItemPublicationLimits::new(
        NonZeroUsize::new(groups).unwrap(),
        NonZeroUsize::new(bytes).unwrap(),
        NonZeroUsize::new(prepare).unwrap(),
    )
    .unwrap()
}

#[test]
fn positive_limits_accept_each_ceiling_and_classify_ceiling_plus_one() {
    for (groups, bytes, prepare, budget) in [
        (
            MAX_ITEM_PUBLICATION_GROUPS,
            1,
            1,
            ItemPublicationBudget::Groups,
        ),
        (
            1,
            MAX_ITEM_PUBLICATION_RETAINED_BYTES,
            1,
            ItemPublicationBudget::RetainedBytes,
        ),
        (
            1,
            1,
            MAX_ITEM_PUBLICATION_PREPARE_BYTES,
            ItemPublicationBudget::PrepareBytes,
        ),
    ] {
        assert!(ItemPublicationLimits::new(
            NonZeroUsize::new(groups).unwrap(),
            NonZeroUsize::new(bytes).unwrap(),
            NonZeroUsize::new(prepare).unwrap()
        )
        .is_ok());
        let (g, b, p) = match budget {
            ItemPublicationBudget::Groups => (groups + 1, bytes, prepare),
            ItemPublicationBudget::RetainedBytes => (groups, bytes + 1, prepare),
            ItemPublicationBudget::PrepareBytes => (groups, bytes, prepare + 1),
        };
        assert!(
            matches!(ItemPublicationLimits::new(NonZeroUsize::new(g).unwrap(),NonZeroUsize::new(b).unwrap(),NonZeroUsize::new(p).unwrap()),Err(ItemPublicationError::InvalidLimits {budget:actual,..}) if actual == budget)
        );
    }
    assert!(matches!(
        publication_charge(Some(usize::MAX)),
        Err(ItemPublicationError::SizeOverflow)
    ));
    assert_eq!(
        publication_charge(None).unwrap(),
        ITEM_PUBLICATION_SCALAR_CHARGE
    );
}

#[test]
fn reserved_sequence_credits_follow_publication_order_and_abort_never_consumes_a_sequence() {
    // Scalar credit arithmetic only: this test does not manufacture a group,
    // origin, receipt or producer authority. Genuine paths are tested separately.
    let mut ledger = ItemPublicationState::new(policy(3, 1024, 1024))
        .unwrap()
        .ledger;
    ledger.last = u64::MAX - 2;
    ledger.check(256).unwrap();
    ledger.reserve(256);
    ledger.check(257).unwrap();
    ledger.reserve(257);
    assert!(matches!(
        ledger.check(258),
        Err(ItemPublicationError::SequenceExhausted)
    ));
    // The later admitted credit confirms first and receives the next actual
    // publication sequence, rather than an admission-time number.
    assert_eq!(ledger.publish(257).get(), u64::MAX - 1);
    assert_eq!(
        (
            ledger.reserved,
            ledger.retained,
            ledger.reserved_bytes,
            ledger.retained_bytes
        ),
        (1, 1, 256, 257)
    );
    ledger.cancel(256);
    assert_eq!(ledger.last, u64::MAX - 1);
    ledger.check(258).unwrap();
    ledger.reserve(258);
    assert_eq!(ledger.publish(258).get(), u64::MAX);
    assert!(matches!(
        ledger.check(1),
        Err(ItemPublicationError::SequenceExhausted)
    ));
    assert_eq!(
        (ledger.reserved, ledger.retained, ledger.retained_bytes),
        (0, 2, 515)
    );
}

#[test]
fn logical_outer_capacity_is_preallocated_and_scalar_checks_are_nonmutating() {
    let state = ItemPublicationState::new(policy(2, 513, 257)).unwrap();
    assert!(state.queue.capacity() >= 2);
    let mut ledger = state.ledger;
    ledger.reserve(256);
    ledger.reserve(257);
    let before = (
        ledger.reserved,
        ledger.retained,
        ledger.reserved_bytes,
        ledger.retained_bytes,
        ledger.last,
    );
    assert!(matches!(
        ledger.check(1),
        Err(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::Groups,
            ..
        })
    ));
    assert_eq!(
        (
            ledger.reserved,
            ledger.retained,
            ledger.reserved_bytes,
            ledger.retained_bytes,
            ledger.last
        ),
        before
    );
    ledger.publish(257);
    assert!(matches!(
        ledger.check(1),
        Err(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::Groups,
            ..
        })
    ));
    ledger.cancel(256);
    ledger.check(256).unwrap();
    assert!(matches!(
        ledger.check(257),
        Err(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::RetainedBytes,
            ..
        })
    ));
    assert!(matches!(
        ledger.check(258),
        Err(ItemPublicationError::Capacity {
            budget: ItemPublicationBudget::RetainedBytes,
            ..
        })
    ));
}
