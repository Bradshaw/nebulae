//! A slightly modified implementation of the iterated Mandelbrot function which, on top of
//! returning whether or not the input value "escapes" within the iteration limit, also returns the
//! list of values from each iteration, necessary for rendering a Nebulabrot

/// Real and imaginary parts of a complex number
#[derive(Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

/// Iterated Mandelbrot function that also returns the points that were traversed during iteration
pub fn iterate(z: Complex, c: Complex, limit: u32, escape: f64, stop: f64) -> (Vec<Complex>, bool) {
    let mut z = z;
    let mut zs: Vec<Complex> = Vec::new();
    let mut escaped = false;
    let escape_squared = escape * escape;
    let stop_squared = stop * stop;

    let mut z2 = Complex {
        re: z.re * z.re,
        im: z.im * z.im,
    };

    let mut iter = 0;

    while (iter < limit) && (z2.re + z2.im < stop_squared) {
        // update z
        z.im = 2.0 * z.re * z.im + c.im;
        z.re = z2.re - z2.im + c.re;

        // update z^2
        z2.re = z.re * z.re;
        z2.im = z.im * z.im;

        // record path
        zs.push(z);

        iter = iter + 1;
    }
    if z2.re + z2.im > escape_squared {
        escaped = true;
    }

    (zs, escaped)
}
