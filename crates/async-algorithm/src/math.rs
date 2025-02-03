// https://en.wikipedia.org/wiki/Methods_of_computing_square_roots
pub fn isqrt(n: u64) -> u64 {
    // X_(n+1)
    let mut x = n;

    // c_n
    let mut c = 0;

    // d_n which starts at the highest power of four <= n
    let mut d = 1u64 << 30; // The second-to-top bit is set.
                            // Same as ((unsigned) INT32_MAX + 1) / 2.
    while d > n {
        d >>= 2;
    }

    // for dₙ … d₀
    while d != 0 {
        if x >= c + d {
            // if X_(m+1) ≥ Y_m then a_m = 2^m
            x -= c + d; // X_m = X_(m+1) - Y_m
            c = (c >> 1) + d; // c_(m-1) = c_m/2 + d_m (a_m is 2^m)
        } else {
            c >>= 1; // c_(m-1) = c_m/2      (aₘ is 0)
        }
        d >>= 2; // d_(m-1) = d_m/4
    }
    c // c_(-1)
}
