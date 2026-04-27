//! Rock entities: the big boulder you click, the small rocks that
//! pop off it, and the dark imprints they leave in the sand.

pub mod big;
pub mod imprint;
pub mod shadow;
pub mod small;

pub use big::BigRockPlugin;
pub use imprint::SandDentPlugin;
pub use shadow::ShadowPlugin;
pub use small::SmallRockPlugin;
