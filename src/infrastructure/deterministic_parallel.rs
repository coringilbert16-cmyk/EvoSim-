//! Deterministic worker execution primitives for simulation calculations.
//!
//! Workers receive immutable, contiguous input slices and return worker-local
//! results. The caller receives results concatenated in original input order,
//! independent of worker completion order. Workers must preserve input order
//! within each chunk and must not mutate authoritative simulation state.
//!
//! This module intentionally does not decide which simulation phases may run
//! concurrently. Snapshot ownership, dependency ordering, transaction commits,
//! and energy-ledger settlement remain the responsibility of the caller.

/// Failure while evaluating one deterministic input chunk.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OrderedChunkError<E> {
    /// A worker returned an application-level error.
    Worker {
        chunk_index: usize,
        source: E,
    },
    /// A worker panicked. All other workers are joined before this is returned.
    Panicked {
        chunk_index: usize,
    },
}

/// Evaluate contiguous chunks on scoped workers and collect their outputs in
/// original input order.
///
/// worker_count is an upper bound. Zero and one both select the sequential
/// reference path. For multiple workers, the input is divided into balanced
/// contiguous chunks; earlier chunks receive at most one extra item.
///
/// The function waits for every spawned worker, even if one fails. It returns
/// no partial output on any worker error or panic. This makes failure explicit
/// and prevents callers from accidentally committing an incomplete calculation.
///
/// Deterministic floating-point reductions must happen after this function
/// returns, over the ordered output, rather than inside worker-local reductions.
pub(crate) fn map_ordered_chunks<T, R, E, F>(
    items: &[T],
    worker_count: usize,
    worker: F,
) -> Result<Vec<R>, OrderedChunkError<E>>
where
    T: Sync,
    R: Send,
    E: Send,
    F: Fn(usize, &[T]) -> Result<Vec<R>, E> + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let chunk_count = worker_count.max(1).min(items.len());
    if chunk_count == 1 {
        return worker(0, items).map_err(|source| OrderedChunkError::Worker {
            chunk_index: 0,
            source,
        });
    }

    let ranges = balanced_chunk_ranges(items.len(), chunk_count);
    std::thread::scope(|scope| {
        let handles: Vec<_> = ranges
            .iter()
            .enumerate()
            .map(|(chunk_index, (start, end))| {
                let input = &items[*start..*end];
                let worker = &worker;
                scope.spawn(move || worker(chunk_index, input))
            })
            .collect();

        // Join every worker before deciding whether the calculation succeeded.
        // Store by chunk index so scheduling and completion order cannot affect
        // reconciliation order or which failure is reported first.
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.join());
        }

        let mut ordered = Vec::new();
        for (chunk_index, result) in results.into_iter().enumerate() {
            let chunk_result = match result {
                Ok(Ok(values)) => values,
                Ok(Err(source)) => {
                    return Err(OrderedChunkError::Worker {
                        chunk_index,
                        source,
                    });
                }
                Err(_) => {
                    return Err(OrderedChunkError::Panicked { chunk_index });
                }
            };
            ordered.extend(chunk_result);
        }

        Ok(ordered)
    })
}

fn balanced_chunk_ranges(item_count: usize, chunk_count: usize) -> Vec<(usize, usize)> {
    debug_assert!(chunk_count > 0);
    debug_assert!(chunk_count <= item_count);

    let base_size = item_count / chunk_count;
    let remainder = item_count % chunk_count;
    let mut start = 0;

    (0..chunk_count)
        .map(|chunk_index| {
            let chunk_size = base_size + usize::from(chunk_index < remainder);
            let end = start + chunk_size;
            let range = (start, end);
            start = end;
            range
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{balanced_chunk_ranges, map_ordered_chunks, OrderedChunkError};

    #[test]
    fn balanced_ranges_cover_every_item_once_and_differ_by_at_most_one() {
        for item_count in 1..40 {
            for chunk_count in 1..=item_count {
                let ranges = balanced_chunk_ranges(item_count, chunk_count);
                assert_eq!(ranges.len(), chunk_count);
                assert_eq!(ranges.first().map(|range| range.0), Some(0));
                assert_eq!(ranges.last().map(|range| range.1), Some(item_count));

                let sizes: Vec<_> = ranges
                    .iter()
                    .map(|(start, end)| end - start)
                    .collect();
                assert!(sizes.iter().all(|size| *size > 0));
                let minimum = *sizes.iter().min().unwrap();
                let maximum = *sizes.iter().max().unwrap();
                assert!(maximum - minimum <= 1);

                for pair in ranges.windows(2) {
                    assert_eq!(pair[0].1, pair[1].0);
                }
            }
        }
    }

    #[test]
    fn parallel_results_are_reconciled_in_input_order() {
        let input: Vec<_> = (0..101).collect();
        let output = map_ordered_chunks(&input, 7, |chunk_index, chunk| {
            let values = chunk
                .iter()
                .map(|value| (chunk_index, *value * 2))
                .collect();
            Ok::<_, ()>(values)
        })
        .unwrap();

        assert_eq!(
            output.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
            input.iter().map(|value| value * 2).collect::<Vec<_>>()
        );
        assert!(output.windows(2).all(|pair| pair[0].1 < pair[1].1));
    }

    #[test]
    fn zero_and_one_worker_use_the_sequential_reference_path() {
        let input = [2, 3, 5, 7];
        for worker_count in [0, 1] {
            let output = map_ordered_chunks(&input, worker_count, |chunk_index, chunk| {
                assert_eq!(chunk_index, 0);
                Ok::<_, ()>(chunk.iter().map(|value| value * value).collect())
            })
            .unwrap();
            assert_eq!(output, vec![4, 9, 25, 49]);
        }
    }

    #[test]
    fn empty_input_does_not_invoke_worker() {
        let output = map_ordered_chunks::<usize, usize, (), _>(&[], 4, |_, _| {
            panic!("worker must not run for empty input");
        })
        .unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn worker_error_returns_no_partial_results() {
        let input: Vec<_> = (0..12).collect();
        let result = map_ordered_chunks(&input, 4, |chunk_index, chunk| {
            if chunk_index == 2 {
                Err("injected worker failure")
            } else {
                Ok(chunk.to_vec())
            }
        });

        assert_eq!(
            result,
            Err(OrderedChunkError::Worker {
                chunk_index: 2,
                source: "injected worker failure"
            })
        );
    }

    #[test]
    fn worker_panic_is_reported_after_all_workers_are_joined() {
        let input: Vec<_> = (0..12).collect();
        let result = map_ordered_chunks(&input, 4, |chunk_index, chunk| {
            if chunk_index == 1 {
                panic!("injected worker panic");
            }
            Ok::<_, ()>(chunk.to_vec())
        });

        assert_eq!(
            result,
            Err(OrderedChunkError::Panicked { chunk_index: 1 })
        );
    }

    #[test]
    fn sequential_and_parallel_paths_reconcile_identically() {
        let input: Vec<_> = (0..257).map(|value| value as f64 / 7.0).collect();
        let calculate = |_, chunk: &[f64]| {
            Ok::<_, ()>(
                chunk
                    .iter()
                    .map(|value| (value.sin(), value.cos()))
                    .collect::<Vec<_>>(),
            )
        };

        let sequential = map_ordered_chunks(&input, 1, calculate).unwrap();
        let parallel = map_ordered_chunks(&input, 8, calculate).unwrap();
        assert_eq!(sequential, parallel);
    }
}
