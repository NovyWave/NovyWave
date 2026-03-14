use crate::selected_variables::SelectedVariableOrGroup;

pub const SELECTED_VARIABLES_GROUP_HEADER_HEIGHT: u32 = 30;
pub const SELECTED_VARIABLE_ROW_DIVIDER_HEIGHT: u32 = 3;
pub const SELECTED_VARIABLES_FOOTER_HEIGHT: u32 = 30;
pub const SELECTED_VARIABLES_EMPTY_CONTENT_HEIGHT: u32 =
    SELECTED_VARIABLES_GROUP_HEADER_HEIGHT + SELECTED_VARIABLES_FOOTER_HEIGHT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedVariablesRowMetric {
    pub row_height: u32,
    pub divider_height_after: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectedVariablesRowSpan {
    pub top_px: f32,
    pub height_px: f32,
    pub divider_height_after_px: f32,
}

impl SelectedVariablesRowMetric {
    pub fn new(row_height: u32, divider_height_after: u32) -> Self {
        Self {
            row_height,
            divider_height_after,
        }
    }

    pub fn group_header() -> Self {
        Self::new(SELECTED_VARIABLES_GROUP_HEADER_HEIGHT, 0)
    }

    pub fn variable(row_height: u32) -> Self {
        Self::new(row_height, SELECTED_VARIABLE_ROW_DIVIDER_HEIGHT)
    }
}

pub fn metrics_from_items<F>(
    items: &[SelectedVariableOrGroup],
    mut row_height_for_variable: F,
) -> Vec<SelectedVariablesRowMetric>
where
    F: FnMut(&str) -> u32,
{
    items
        .iter()
        .map(|item| match item {
            SelectedVariableOrGroup::GroupHeader { .. } => {
                SelectedVariablesRowMetric::group_header()
            }
            SelectedVariableOrGroup::Variable(variable) => {
                SelectedVariablesRowMetric::variable(row_height_for_variable(&variable.unique_id))
            }
        })
        .collect()
}

pub fn total_content_height(metrics: &[SelectedVariablesRowMetric]) -> u32 {
    if metrics.is_empty() {
        return SELECTED_VARIABLES_EMPTY_CONTENT_HEIGHT;
    }

    metrics
        .iter()
        .fold(SELECTED_VARIABLES_FOOTER_HEIGHT, |total, metric| {
            total
                .saturating_add(metric.row_height)
                .saturating_add(metric.divider_height_after)
        })
}

pub fn compute_row_spans(metrics: &[SelectedVariablesRowMetric]) -> Vec<SelectedVariablesRowSpan> {
    let mut top_px = 0.0f32;
    let mut spans = Vec::with_capacity(metrics.len());

    for metric in metrics {
        spans.push(SelectedVariablesRowSpan {
            top_px,
            height_px: metric.row_height as f32,
            divider_height_after_px: metric.divider_height_after as f32,
        });
        top_px += metric.row_height as f32 + metric.divider_height_after as f32;
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::{
        SELECTED_VARIABLES_FOOTER_HEIGHT, SelectedVariablesRowMetric, compute_row_spans,
        total_content_height,
    };

    #[test]
    fn total_height_includes_variable_dividers_and_footer() {
        let metrics = vec![
            SelectedVariablesRowMetric::variable(30),
            SelectedVariablesRowMetric::group_header(),
            SelectedVariablesRowMetric::variable(90),
        ];

        assert_eq!(
            total_content_height(&metrics),
            SELECTED_VARIABLES_FOOTER_HEIGHT + 30 + 3 + 30 + 90 + 3
        );
    }

    #[test]
    fn row_spans_accumulate_divider_offsets() {
        let metrics = vec![
            SelectedVariablesRowMetric::variable(30),
            SelectedVariablesRowMetric::group_header(),
            SelectedVariablesRowMetric::variable(70),
        ];

        let spans = compute_row_spans(&metrics);

        assert_eq!(spans[0].top_px, 0.0);
        assert_eq!(spans[1].top_px, 33.0);
        assert_eq!(spans[2].top_px, 63.0);
    }
}
