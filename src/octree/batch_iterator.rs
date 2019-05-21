use crate::errors::*;
use crate::math::{Isometry3, Obb, OrientedBeam};
use crate::octree::{self, Octree};
use crate::{LayerData, Point, PointData};
use cgmath::{Decomposed, Matrix4, Vector3, Vector4};
use collision::Aabb3;
use fnv::FnvHashMap;

/// size for batch
pub const NUM_POINTS_PER_BATCH: usize = 500_000;

///possible kind of iterators that can be evaluated in batch of points in BatchIterator
#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum PointCulling {
    Any(),
    Aabb(Aabb3<f64>),
    Obb(Obb<f64>),
    Frustum(Matrix4<f64>),
    OrientedBeam(OrientedBeam),
}

pub struct PointLocation {
    pub culling: PointCulling,
    // If set, culling and the returned points are interpreted to be in local coordinates.
    pub global_from_local: Option<Isometry3<f64>>,
}

/// current implementation of the stream of points used in BatchIterator
struct PointStream<'a, F>
where
    F: FnMut(PointData) -> Result<()>,
{
    position: Vec<Vector3<f64>>,
    color: Vec<Vector4<u8>>,
    intensity: Vec<f32>,
    local_from_global: Option<Isometry3<f64>>,
    func: &'a mut F,
}

impl<'a, F> PointStream<'a, F>
where
    F: FnMut(PointData) -> Result<()>,
{
    fn new(
        num_points_per_batch: usize,
        local_from_global: Option<Isometry3<f64>>,
        func: &'a mut F,
    ) -> Self {
        PointStream {
            position: Vec::with_capacity(num_points_per_batch),
            color: Vec::with_capacity(num_points_per_batch),
            intensity: Vec::with_capacity(num_points_per_batch),
            local_from_global,
            func,
        }
    }

    /// push point in batch
    fn push_point(&mut self, point: Point) {
        let position = match &self.local_from_global {
            Some(local_from_global) => local_from_global * &point.position,
            None => point.position,
        };
        self.position.push(position);
        self.color.push(Vector4::new(
            point.color.red,
            point.color.green,
            point.color.blue,
            point.color.alpha,
        ));
        if let Some(point_intensity) = point.intensity {
            self.intensity.push(point_intensity);
        };
    }

    /// execute function on batch of points
    fn callback(&mut self) -> Result<()> {
        if self.position.is_empty() {
            return Ok(());
        }

        let mut layers = FnvHashMap::default();
        layers.insert(
            "color".to_string(),
            LayerData::U8Vec4(self.color.split_off(0)),
        );
        if !self.intensity.is_empty() {
            layers.insert(
                "intensity".to_string(),
                LayerData::F32(self.intensity.split_off(0)),
            );
        }
        let point_data = PointData {
            position: self.position.split_off(0),
            layers,
        };
        (self.func)(point_data)
    }

    fn push_point_and_callback(&mut self, point: Point) -> Result<()> {
        self.push_point(point);
        if self.position.len() == self.position.capacity() {
            return self.callback();
        }
        Ok(())
    }
}

/// Iterator on point batches
pub struct BatchIterator<'a> {
    octree: &'a Octree,
    culling: PointCulling,
    local_from_global: Option<Isometry3<f64>>,
    batch_size: usize,
}

impl<'a> BatchIterator<'a> {
    pub fn new(octree: &'a octree::Octree, location: &'a PointLocation, batch_size: usize) -> Self {
        let culling = match &location.global_from_local {
            Some(global_from_local) => match &location.culling {
                PointCulling::Any() => PointCulling::Any(),
                PointCulling::Aabb(aabb) => {
                    PointCulling::Obb(Obb::from(*aabb).transform(global_from_local))
                }
                PointCulling::Obb(obb) => PointCulling::Obb(obb.transform(global_from_local)),
                PointCulling::Frustum(frustum) => PointCulling::Frustum(
                    Matrix4::from(Decomposed {
                        scale: 1.0,
                        rot: global_from_local.rotation,
                        disp: global_from_local.translation,
                    }) * frustum,
                ),
                PointCulling::OrientedBeam(beam) => {
                    PointCulling::OrientedBeam(beam.transform(global_from_local))
                }
            },
            None => location.culling.clone(),
        };
        let local_from_global = location.global_from_local.clone().map(|t| t.inverse());
        BatchIterator {
            octree,
            culling,
            local_from_global,
            batch_size,
        }
    }

    /// compute a function while iterating on a batch of points
    pub fn try_for_each_batch<F>(&mut self, mut func: F) -> Result<()>
    where
        F: FnMut(PointData) -> Result<()>,
    {
        let mut point_stream =
            PointStream::new(self.batch_size, self.local_from_global.clone(), &mut func);
        let mut iterator: Box<Iterator<Item = Point>> = match &self.culling {
            PointCulling::Any() => Box::new(self.octree.all_points()),
            PointCulling::Aabb(aabb) => Box::new(self.octree.points_in_box(aabb)),
            PointCulling::Obb(obb) => Box::new(self.octree.points_in_obb(obb)),
            PointCulling::Frustum(frustum) => Box::new(self.octree.points_in_frustum(frustum)),
            PointCulling::OrientedBeam(beam) => Box::new(self.octree.points_in_oriented_beam(beam)),
        };
        iterator.try_for_each(|point: Point| point_stream.push_point_and_callback(point))?;
        point_stream.callback()
    }
}