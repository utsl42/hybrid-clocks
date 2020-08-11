use utsl_hybrid_clocks::{Clock, ManualClock, Result, Timestamp};
use suppositions::generators::*;
use suppositions::*;

fn observing<'a>(clock: &mut Clock<ManualClock>, msg: &Timestamp<u64>) -> Result<Timestamp<u64>> {
    clock.observe(msg);
    Ok(clock.now()?)
}

pub fn timestamps<C: Generator + 'static>(
    times: C,
) -> Box<dyn GeneratorObject<Item = Timestamp<C::Item>>> {
    let counts = u16s();
    (times, counts)
        .map(|(time, count)| Timestamp { time, count })
        .boxed()
}

#[test]
fn fig_6_proc_0_a() -> Result<()> {
    let mut clock = Clock::manual(0)?;
    clock.set_time(10);
    assert_eq!(
        clock.now()?,
        Timestamp {
            time: 10,
            count: 0
        }
    );
    Ok(())
}

#[test]
fn fig_6_proc_1_a() -> Result<()> {
    let mut clock = Clock::manual(1)?;
    assert_eq!(
        observing(
            &mut clock,
            &Timestamp {
                time: 10,
                count: 0
            }
        )
        .unwrap(),
        Timestamp {
            time: 10,
            count: 1
        }
    );
    Ok(())
}

#[test]
fn fig_6_proc_1_b() -> Result<()> {
    let mut clock = Clock::manual(1)?;
    let _ = observing(
        &mut clock,
        &Timestamp {
            time: 10,
            count: 0,
        },
    )
    .unwrap();
    clock.set_time(2);
    assert_eq!(
        clock.now()?,
        Timestamp {
            time: 10,
            count: 2
        }
    );
    Ok(())
}

#[test]
fn fig_6_proc_2_b() -> Result<()> {
    let mut clock = Clock::manual(0)?;
    clock.observe(&Timestamp {
        time: 1,
        count: 0,
    });

    clock.set_time(2);
    assert_eq!(
        observing(
            &mut clock,
            &Timestamp {
                time: 10,
                count: 2
            }
        )
        .unwrap(),
        Timestamp {
            time: 10,
            count: 3
        }
    );
    Ok(())
}

#[test]
fn fig_6_proc_2_c() -> Result<()> {
    let mut clock = Clock::manual(0)?;
    clock.set_time(2);
    let _ = observing(
        &mut clock,
        &Timestamp {
            time: 10,
            count: 2,
        },
    )
    .unwrap();
    clock.set_time(3);
    assert_eq!(
        clock.now()?,
        Timestamp {
            time: 10,
            count: 4
        }
    );
    Ok(())
}

#[test]
fn all_sources_same() -> Result<()> {
    let mut clock = Clock::manual(0)?;
    let observed = Timestamp {
        time: 0,
        count: 5,
    };
    let result = observing(&mut clock, &observed)?;
    println!("obs:{:?}; result:{:?}", observed, result);
    assert!(result > observed);
    assert!(result.time == observed.time);
    Ok(())
}

#[test]
fn handles_time_going_backwards_now() -> Result<()> {
    let mut clock = Clock::manual(10)?;
    let _ = clock.now()?;
    clock.set_time(9);
    assert_eq!(
        clock.now()?,
        Timestamp {
            time: 10,
            count: 2
        }
    );
    Ok(())
}

#[test]
fn handles_time_going_backwards_observe() -> Result<()> {
    let mut clock = Clock::manual(10)?;
    let original = clock.now()?;
    clock.set_time(9);
    let result = observing(
        &mut clock,
        &Timestamp {
            time: 0,
            count: 0,
        },
    )
    .unwrap();
    assert!(result > original);
    assert!(result.time == 10);
    Ok(())
}

#[test]
fn handles_time_going_forwards_now() -> Result<()> {
    let mut clock = Clock::manual(10)?;
    let t = clock.now()?;
    println!("at 10: {}", t);
    clock.set_time(12);
    let t2 = clock.now()?;
    println!("=> 12: {}", t2);
    assert_eq!(
        t2,
        Timestamp {
            time: 12,
            count: 0
        }
    );
    Ok(())
}

#[test]
fn handles_time_going_forwards_observe() -> Result<()> {
    let mut clock = Clock::manual(10)?;
    let _ = clock.now()?;
    clock.set_time(12);
    assert_eq!(
        observing(
            &mut clock,
            &Timestamp {
                time: 0,
                count: 0
            }
        )
        .unwrap(),
        Timestamp {
            time: 12,
            count: 0
        }
    );
    Ok(())
}

#[test]
fn supposedly_be_larger_than_observed_time() -> Result<()> {
    property((u64s(), timestamps(u64s()))).check(|(t0, advanced_epoch)| -> Result<()> {
        let mut clock0 = Clock::manual(t0)?;
        let t2 = observing(&mut clock0, &advanced_epoch).unwrap();
        println!("t0: {:?}; 👀: {:?} => {:?}", t0, advanced_epoch, t2);
        assert!(t2 > advanced_epoch, "{:?} > {:?}", t2, advanced_epoch);
        Ok(())
    });
    Ok(())
}

#[test]
fn supposedly_be_larger_than_observed_clock() -> Result<()> {
    property((u64s(), timestamps(u64s()))).check(|(t0, advanced_epoch)| -> Result<()> {
        let mut clock0 = Clock::manual(t0)?;
        let t1 = clock0.now()?;
        let t2 = observing(&mut clock0, &advanced_epoch).unwrap();
        println!("t0: {:?}; 👀: {:?} => {:?}", t0, advanced_epoch, t2);
        assert!(t2 > t1, "{:?} > {:?}", t2, t1);
        Ok(())
    });
    Ok(())
}

#[test]
fn should_ignore_clocks_too_far_forward() -> Result<()> {
    let src = ManualClock::new(0);
    let mut clock = Clock::new(src)?.with_max_diff(10);
    assert!(clock
        .observe(&Timestamp {
            time: 11,
            count: 0
        })
        .is_err());

    clock
        .observe(&Timestamp {
            time: 1,
            count: 0,
        })
        .unwrap();
    assert_eq!(
        clock.now().expect("now"),
        Timestamp {
            time: 1,
            count: 1
        }
    );
    Ok(())
}

#[test]
fn should_account_for_time_passing_when_checking_max_error() -> Result<()> {
    let src = ManualClock::new(0);
    let mut clock = Clock::new(src)?.with_max_diff(10);
    clock.inner_mut().set_time(1);

    assert!(clock
        .observe(&Timestamp {
            time: 11,
            count: 0
        })
        .is_ok());
    Ok(())
}

// Unsigned overflow made `clock.observe(past_timestamp)` to return an
// `OffsetTooGreat` error in release mode and a panic in debug mode.
#[test]
fn should_observe_past_timestamp() -> Result<()> {
    let src = ManualClock::new(10);
    let mut clock = Clock::new(src)?.with_max_diff(2);

    assert!(clock
        .observe(&Timestamp {
            time: 9,
            count: 0
        })
        .is_ok());
    Ok(())
}

