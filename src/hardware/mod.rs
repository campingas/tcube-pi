pub mod button;
// The GPIO listener only runs on the Pi backend; on other targets the module
// still compiles (and its core is unit-tested) but nothing outside tests uses
// it, so the dead code lint is scoped out there and enforced on Linux CI.
#[cfg_attr(not(all(feature = "pi-gpio", target_os = "linux")), allow(dead_code))]
pub mod gpio;
pub mod soundbox;
