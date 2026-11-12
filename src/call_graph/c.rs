//! C and C++ call-graph-layer facts shared by later layers (#630 slice 6).

pub mod macro_census;

#[cfg(test)]
mod macro_census_tests;

#[cfg(test)]
mod macro_census_fold2_tests;
#[cfg(test)]
mod macro_census_fold3_tests;
#[cfg(test)]
mod macro_census_s6b_fold3_tests;
#[cfg(test)]
mod macro_census_s6b_tests;
