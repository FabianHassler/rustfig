//! The bridge between RustFig's document model and fig2dev's objects (plan
//! D1): [`to_model`] turns what fig2dev's reader returns into a
//! [`rustfig_model::Document`]; [`from_model`] builds fig2dev's objects
//! and [`fig2dev::Ctx`] from a document without writing and parsing Fig
//! text.
//!
//! [`from_model::from_model`] gives the objects as `read.c` returns them
//! for the Fig 3.2 text [`crate::write_fig`] writes (same rounding, same
//! closed-path points, same arrowhead rules, colours renumbered the way the
//! reader renumbers them), except that splines stay splines with their
//! shape factors, so that [`to_model::to_document`] gets the document back.
//! Hand them to fig2dev's drivers through [`from_model::driver_objects`],
//! which converts the splines to lines as the reader does.

pub mod from_model;
pub mod to_model;

pub use from_model::{driver_objects, from_model, note_usage, object_to_fig2dev, FigObject};
pub use to_model::to_document;
