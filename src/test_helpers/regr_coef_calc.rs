pub async fn find_average_min<F, ForkIt, FactIt>(
    success: impl Fn(usize, f32) -> F,
    fork: ForkIt,
    factor: FactIt,
    samples: usize,
) -> Vec<(usize, f32)>
where
    F: Future<Output = bool>,
    ForkIt: IntoIterator<Item = usize> + Clone,
    FactIt: IntoIterator<Item = f32> + Clone,
{
    let mut results = Vec::new();
    for n_forks in fork {
        let mut values: Vec<f32> = Vec::new();

        for _ in 0..samples {
            for f in factor.clone() {
                let success = success(n_forks, f).await;
                if success {
                    values.push(f);
                    break;
                }
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let n_values: f32 = values.len() as f32;

        results.push((n_forks, values.iter().sum::<f32>() / n_values));
    }

    results
}

pub fn floats_from_to(from: f32, to: f32, step: f32) -> impl Iterator<Item = f32> + Clone {
    let mut current = from;
    std::iter::from_fn(move || {
        if current > to {
            None
        } else {
            let value = current;
            current += step;
            Some(value)
        }
    })
}

pub fn ints_from_to(from: usize, to: usize, step: usize) -> impl Iterator<Item = usize> + Clone {
    let mut current = from;
    std::iter::from_fn(move || {
        if current > to {
            None
        } else {
            let value = current;
            current += step;
            Some(value)
        }
    })
}
