use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Query,
    query::{QueryItem, QueryState, With},
    world::FromWorld,
};
use bevy_ecs::{
    system::{Commands, Res, Resource},
    world::World,
};
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, ViewNode},
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ExtractedWindows, ViewTarget},
    Extract,
};
use bevy_window::Window;
use iced_runtime::core::Size;
use iced_wgpu::graphics::Viewport;

use crate::{DidDraw, IcedProps, IcedResource, IcedSettings};

/// Iced render pass
pub const ICED_PASS: &str = "bevy_iced_pass";

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct ViewportResource(pub Viewport);

pub(crate) fn update_viewport(
    windows: Query<&Window>,
    iced_settings: Res<IcedSettings>,
    mut commands: Commands,
) {
    let window = windows.single();
    let scale_factor = iced_settings.scale_factor.unwrap_or(window.scale_factor());
    let viewport = Viewport::with_physical_size(
        Size::new(window.physical_width(), window.physical_height()),
        scale_factor,
    );
    commands.insert_resource(ViewportResource(viewport));
}

// Same as DidDraw, but as a regular bool instead of an atomic.
#[derive(Resource, Deref, DerefMut)]
struct DidDrawBasic(bool);

pub(crate) fn extract_iced_data(
    mut commands: Commands,
    viewport: Extract<Res<ViewportResource>>,
    did_draw: Extract<Res<DidDraw>>,
) {
    commands.insert_resource(viewport.clone());
    commands.insert_resource(DidDrawBasic(
        did_draw.swap(false, std::sync::atomic::Ordering::Relaxed),
    ));
}

/// Iced node
pub struct IcedNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
}

impl FromWorld for IcedNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl ViewNode for IcedNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    type ViewQuery = &'static ViewTarget;

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // let Some(extracted_window) = world
        //     .get_resource::<ExtractedWindows>()
        //     .unwrap()
        //     .windows
        //     .values()
        //     .next() else { return Ok(()) };

        let view_entity = graph.view_entity();

        let Ok(view_target) = self.query.get_manual(world, view_entity) else {
            return Ok(())
        };

        let IcedProps {
            renderer, debug, ..
        } = &mut *world.resource::<IcedResource>().lock().unwrap();
        let render_device = world.resource::<RenderDevice>();
        let queue = world.resource::<RenderQueue>();

        if !world
            .get_resource::<DidDrawBasic>()
            .map(|x| x.0)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let view = view_target.main_texture_view();

        let viewport = world.resource::<ViewportResource>();
        let device = render_device.wgpu_device();

        let iced_renderer::Renderer::Wgpu(renderer) = renderer else { return Ok(()); };
        renderer.with_primitives(|backend, primitives| {
            backend.present(
                device,
                queue,
                render_context.command_encoder(),
                None,
                view,
                primitives,
                viewport,
                &debug.overlay(),
            );
        });

        Ok(())
    }
}
