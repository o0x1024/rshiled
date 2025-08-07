use rand_distr::Uniform;
use rand::{rng, Rng};

pub(crate) fn get_rand_bytes(len: usize) -> Vec<u8> {
    let rand_string: Vec<String> = rng()
        .sample_iter(&Uniform::new(0, 9).unwrap())
        .take(len)
        .map(|x| x.to_string())
        .collect();
    return rand_string.join("").as_bytes().to_vec();
}
