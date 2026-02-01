// This file is derived from the 'exbibyte/dt' crate
// Original source: https://github.com/exbibyte/dt
// Commit: a590c0b0f9eda4f99669e53a57a9b782aa166272
//
// Original MIT License:
//
// Copyright (c) 2020 YL
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use ndarray::prelude::*;
use num::Float;

/// Compute shortest euclidean distance of points to masked regions.
/// Assumes masked regions have boolean value of true
pub fn dt_bool<T: num::Float>(a: &Array<bool, IxDyn>) -> Array<T, IxDyn> {
    let mut ret = Array::<T, IxDyn>::zeros(a.shape());

    let it = a.iter();
    let it2 = ret.iter_mut();

    for (&orig, new) in it.zip(it2) {
        *new = if orig { T::one() } else { T::zero() };
    }

    dt(&ret)
}

/// Compute shortest euclidean distance of points to masked regions.
/// Assumes masked regions have int value of non-zero
pub fn dt_int<T: num::Float, U: num::Integer>(a: &Array<U, IxDyn>) -> Array<T, IxDyn> {
    let mut ret = Array::<T, IxDyn>::zeros(a.shape());

    let it = a.iter();
    let it2 = ret.iter_mut();

    for (orig, new) in it.zip(it2) {
        *new = if orig != &U::zero() {
            T::one()
        } else {
            T::zero()
        };
    }

    dt(&ret)
}

/// Compute shortest euclidean distance of points to masked regions.
/// Assumes masked regions have values > 0.5
pub fn dt<T: num::Float>(a: &Array<T, IxDyn>) -> Array<T, IxDyn> {
    use std::cmp::max;

    let mut ret = a.clone();

    let mut dim_max = 0;
    for i in ret.shape() {
        dim_max = max(*i, dim_max);
    }

    let mut f: Vec<T> = vec![T::zero(); dim_max];

    for (idx, _) in a.shape().iter().enumerate() {
        let l = ret.lanes_mut(Axis(idx));

        for mut j in l {
            let s = j.len();
            for k in 0..s {
                if idx == 0 {
                    f[k] = if j[k] > num::cast(0.5).unwrap() {
                        T::zero()
                    } else {
                        num::cast(1e37).unwrap()
                    };
                } else {
                    f[k] = j[k];
                }
            }
            let mut d: Vec<T> = vec![T::zero(); s];
            dt_1d(&mut d, &mut f, s);
            for i in 0..s {
                j[i] = d[i];
            }
        }
    }

    //l2 distance
    let it = ret.iter_mut();
    for x in it {
        *x = x.sqrt();
    }
    ret
}

fn dt_1d<T: num::Float>(out: &mut Vec<T>, f: &Vec<T>, n: usize) {
    debug_assert!(out.len() >= n);

    let mut v: Vec<usize> = vec![0; n];
    let mut z: Vec<T> = vec![T::zero(); n + 1];
    let mut k = 0;
    z[0] = Float::neg_infinity();
    z[1] = Float::infinity();

    for q in 1..=n - 1 {
        let qq: T = num::cast(q * q).unwrap();

        let mut s: T = (f[q] + qq - (f[v[k]] + num::cast(v[k] * v[k]).unwrap()))
            / num::cast(2 * q - 2 * v[k]).unwrap();

        while !s.is_infinite() && s <= z[k] {
            k -= 1;
            let vv: T = num::cast(v[k] * v[k]).unwrap();
            s = (f[q] + qq - (f[v[k]] + vv)) / num::cast(2 * q - 2 * v[k]).unwrap();
        }
        k += 1;
        debug_assert!(k < v.len());
        v[k] = q;
        debug_assert!(k < z.len());
        z[k] = s;
        debug_assert!(k + 1 < z.len());
        z[k + 1] = Float::infinity();
    }
    k = 0;
    for q in 0..=n - 1 {
        while z[k + 1] < num::cast(q).unwrap() {
            k += 1;
        }
        out[q] =
            (num::cast::<_, T>(q).unwrap() - num::cast::<_, T>(v[k]).unwrap()).powi(2) + f[v[k]];
    }
}
