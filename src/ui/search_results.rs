use gpui::{
    Context, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    SharedString, Styled, Window, div,
};
use gpui_component::ActiveTheme;

#[derive(Clone)]
pub struct SearchResultsList {
    pub(crate) results: Vec<crate::apps::App>,
}

impl SearchResultsList {
    #[must_use]
    pub fn new(results: Vec<crate::apps::App>) -> Self {
        Self { results }
    }
}

impl Render for SearchResultsList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("apps-list")
            .size_full()
            .flex()
            .flex_col()
            .children(self.results.iter().enumerate().map(|(i, app)| {
                let app_name = SharedString::from(app.name.clone());
                let path = app.path.clone();
                div()
                    .id(ElementId::named_usize(app_name.clone(), i))
                    .p_0p5()
                    .hover(|style| style.bg(cx.theme().secondary_hover))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.open_with_system(path.as_path());
                        window.remove_window();
                    })
                    .child(app_name)
            }))
    }
}
