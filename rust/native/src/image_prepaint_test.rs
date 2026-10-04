//! TestPlatform phase ordering, not native GPU or desktop acceptance.
use super::*;
use gpui::{AppContext, Render, TestAppContext, div};
use std::{cell::Cell, os::fd::AsRawFd, os::unix::net::UnixStream};

struct Fixture {
    binding: Rc<RefCell<Binding>>,
    owner: gpui::WeakEntity<View>,
    width: Rc<Cell<f32>>,
    avatar: bool,
    before_paint: Rc<RefCell<Vec<Option<ImageError>>>>,
}

impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let binding = self.binding.clone();
        let before_paint = self.before_paint.clone();
        div()
            .w(px(self.width.get()))
            .h(px(64.))
            .child(vector(
                &self.binding,
                ImageFit::Cover,
                false,
                Default::default(),
                self.avatar.then(|| Arc::from("?")),
                self.owner.clone(),
                ImageState::Failed(ImageError::InvalidData),
            ))
            // This sibling runs after the SVG's prepaint but before *any* paint.
            // Observing only the final frame would miss the late-selection bug.
            .child(
                canvas(
                    move |_, _, _| {
                        before_paint
                            .borrow_mut()
                            .push(binding.borrow().layout_error);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size(px(1.)),
            )
    }
}

#[test]
fn avatar_size_failure_and_recovery_are_available_before_paint() {
    for avatar in [true, false] {
        let mut app = TestAppContext::single();
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(crate::transport::Transport::new(writer.as_raw_fd()).unwrap());
        let owner = app.new(|_| {
            View::new(
                gpuio_protocol::WindowId::from_parts(0, 1).unwrap(),
                Rc::new(RefCell::new(crate::session::Session::default())),
                transport,
            )
        });
        // This fixture observes phase ordering, not bridge delivery. Retire the
        // notification recipient so its deliberately fixed render status cannot
        // create a synthetic redraw loop in TestPlatform's run-until-parked step.
        let owner = {
            let weak = owner.downgrade();
            drop(owner);
            weak
        };
        // No worker or image upload is needed to test measured-geometry admission.
        // The same production vector primitive handles source and layout errors.
        let binding = Rc::new(RefCell::new(Binding::new(
            Err(ImageError::InvalidData),
            false,
        )));
        let width = Rc::new(Cell::new(20000.));
        let before_paint = Rc::new(RefCell::new(Vec::new()));
        let (view, cx) = app.add_window_view(|_, _| Fixture {
            binding: binding.clone(),
            owner,
            width: width.clone(),
            avatar,
            before_paint: before_paint.clone(),
        });
        // add_window_view may already draw. Reset the error so this explicitly
        // forced frame establishes the first failure, not an eventual retry.
        binding.borrow_mut().layout_error = None;
        before_paint.borrow_mut().clear();
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            before_paint.borrow().first().copied(),
            Some(avatar.then_some(ImageError::ResourceLimit)),
            "avatar={avatar}: slot selection needs the first failing frame"
        );
        assert_eq!(
            binding.borrow().layout_error,
            Some(ImageError::ResourceLimit)
        );

        width.set(64.);
        before_paint.borrow_mut().clear();
        view.update(cx, |_, cx| cx.notify());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            before_paint.borrow().first().copied(),
            Some((!avatar).then_some(ImageError::ResourceLimit)),
            "avatar={avatar}: recovery must clear the error before slot selection"
        );
        assert_eq!(binding.borrow().layout_error, None);
    }
}
