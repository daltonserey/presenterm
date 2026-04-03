use super::properties::WindowSize;
use crate::{
    ImagePrinter,
    presentation::Presentation,
    render::{
        RenderError,
        engine::{RenderEngine, RenderEngineOptions},
        operation::RenderOperation,
    },
    terminal::{Terminal, TerminalWrite},
};
use std::{io, sync::Arc};

pub(crate) struct OverflowValidator;

impl OverflowValidator {
    pub(crate) fn validate(presentation: &Presentation, dimensions: WindowSize) -> Result<(), OverflowError> {
        let printer = Arc::new(ImagePrinter::Null);
        for (index, slide) in presentation.iter_slides().enumerate() {
            let index = index + 1;
            let mut terminal = Terminal::new(io::Empty::default(), printer.clone()).map_err(RenderError::from)?;
            let options = RenderEngineOptions { validate_overflows: true, ..Default::default() };
            let engine = RenderEngine::new(&mut terminal, dimensions, options);
            match engine.render(slide.iter_visible_operations()) {
                Ok(()) => (),
                Err(RenderError::HorizontalOverflow) => return Err(OverflowError::Horizontal(index)),
                Err(RenderError::VerticalOverflow) => return Err(OverflowError::Vertical(index)),
                Err(e) => return Err(OverflowError::Render(e)),
            };
        }
        Ok(())
    }

    pub(crate) fn find_split_point(operations: &[RenderOperation], dimensions: WindowSize) -> Option<usize> {
        let printer = Arc::new(ImagePrinter::Null);
        let mut terminal = Terminal::new(io::Empty::default(), printer).map_err(RenderError::from).ok()?;
        let options = RenderEngineOptions { validate_overflows: true, max_size: Default::default() };
        let mut engine = RenderEngine::new(&mut terminal, dimensions, options);

        for (index, operation) in operations.iter().enumerate() {
            match engine.render_one(operation) {
                Ok(()) => {
                    if engine.max_row() > dimensions.rows {
                        return Some(index);
                    }
                }
                Err(RenderError::VerticalOverflow) => {
                    return Some(index);
                }
                Err(_) => break,
            }
        }
        None
    }

    pub(crate) fn find_logical_split_point(operations: &[RenderOperation], split_index: usize) -> usize {
        let operations_slice = &operations[..split_index];
        for i in (0..split_index).rev() {
            if is_logical_split_point(operations_slice, i) {
                return i + 1;
            }
        }
        split_index
    }
}

fn is_logical_split_point(operations: &[RenderOperation], index: usize) -> bool {
    let op = &operations[index];
    match op {
        RenderOperation::RenderLineBreak => {
            if index > 0 {
                let prev = &operations[index - 1];
                matches!(prev, RenderOperation::RenderLineBreak)
            } else {
                false
            }
        }
        RenderOperation::RenderBlockLine(_) => true,
        RenderOperation::RenderImage(_, _) => true,
        RenderOperation::InitColumnLayout { .. } => true,
        _ => false,
    }
}

impl TerminalWrite for io::Empty {
    fn init(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn deinit(&mut self) {}
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OverflowError {
    #[error("presentation overflows horizontally on slide {0}")]
    Horizontal(usize),

    #[error("presentation overflows vertically on slide {0}")]
    Vertical(usize),

    #[error(transparent)]
    Render(#[from] RenderError),
}
