use crate::{
    context::Context,
    types::{GaugeDraw, GaugeInstall, SystemInstall},
};

pub trait System: 'static {
    fn init(&mut self, ctx: &Context, install: &SystemInstall) -> bool;
    fn update(&mut self, ctx: &Context, dt: f32) -> bool;
    fn kill(&mut self, ctx: &Context) -> bool;
}

pub trait Gauge: 'static {
    fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool;
    fn update(&mut self, ctx: &Context, dt: f32) -> bool;
    fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool;
    fn kill(&mut self, ctx: &Context) -> bool;

    fn mouse(&mut self, _ctx: &Context, _x: f32, _y: f32, _flags: i32) {}
}
