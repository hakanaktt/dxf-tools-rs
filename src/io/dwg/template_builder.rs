//! DWG Template Builder - Converts CadTemplate objects to actual entities
//!
//! This module handles the conversion of intermediate template objects (read from
//! the DWG binary) into proper entity objects that can be used in a CadDocument.

use std::collections::HashMap;
use crate::types::{Handle, Vector2, Vector3};
use crate::entities::{
    EntityType, EntityCommon,
    Line, Circle, Arc, Point, Ellipse, Text, MText,
    LwPolyline, LwVertex, Spline, SplineFlags,
    Insert, Solid, Ray, XLine,
};
use super::object_reader::{CadTemplate, DwgEntityData};

/// Handle map for resolving handle references
pub type HandleMap = HashMap<u64, Handle>;

/// Layer map for resolving layer references  
pub type LayerMap = HashMap<u64, String>;

/// Builder for converting DWG templates to entities
pub struct DwgTemplateBuilder {
    /// Handle map: DWG handle -> Document handle
    #[allow(dead_code)]
    handle_map: HandleMap,
    /// Layer name map: layer handle -> layer name
    layer_map: LayerMap,
}

impl DwgTemplateBuilder {
    /// Create a new template builder
    pub fn new() -> Self {
        Self {
            handle_map: HashMap::new(),
            layer_map: HashMap::new(),
        }
    }
    
    /// Set the layer map for resolving layer references
    pub fn with_layer_map(mut self, layers: LayerMap) -> Self {
        self.layer_map = layers;
        self
    }
    
    /// Set the handle map for resolving references
    pub fn with_handle_map(mut self, handles: HandleMap) -> Self {
        self.handle_map = handles;
        self
    }
    
    /// Convert common entity data to EntityCommon
    fn build_entity_common(&self, data: &DwgEntityData) -> EntityCommon {
        let mut common = EntityCommon::default();
        
        common.handle = Handle::new(data.handle);
        common.color = data.color.clone();
        common.invisible = data.invisible;
        
        // Resolve layer name
        if let Some(layer_handle) = data.layer_handle {
            if let Some(layer_name) = self.layer_map.get(&layer_handle) {
                common.layer = layer_name.clone();
            }
        }
        
        common
    }
    
    /// Convert a template to an entity
    pub fn build_entity(&self, template: &CadTemplate) -> Option<EntityType> {
        match template {
            CadTemplate::Line { entity_data, start, end, thickness, extrusion } => {
                let mut line = Line::new();
                line.common = self.build_entity_common(entity_data);
                line.start = *start;
                line.end = *end;
                line.thickness = *thickness;
                line.normal = *extrusion;
                Some(EntityType::Line(line))
            }
            
            CadTemplate::Circle { entity_data, center, radius, thickness, extrusion } => {
                let mut circle = Circle::new();
                circle.common = self.build_entity_common(entity_data);
                circle.center = *center;
                circle.radius = *radius;
                circle.thickness = *thickness;
                circle.normal = *extrusion;
                Some(EntityType::Circle(circle))
            }
            
            CadTemplate::Arc { entity_data, center, radius, thickness, extrusion, start_angle, end_angle } => {
                let mut arc = Arc::new();
                arc.common = self.build_entity_common(entity_data);
                arc.center = *center;
                arc.radius = *radius;
                arc.thickness = *thickness;
                arc.normal = *extrusion;
                arc.start_angle = *start_angle;
                arc.end_angle = *end_angle;
                Some(EntityType::Arc(arc))
            }
            
            CadTemplate::Point { entity_data, location, thickness, extrusion, .. } => {
                let mut point = Point::at(*location);
                point.common = self.build_entity_common(entity_data);
                point.thickness = *thickness;
                point.normal = *extrusion;
                Some(EntityType::Point(point))
            }
            
            CadTemplate::Ellipse { entity_data, center, major_axis, extrusion, axis_ratio, start_angle, end_angle } => {
                let mut ellipse = Ellipse::new();
                ellipse.common = self.build_entity_common(entity_data);
                ellipse.center = *center;
                ellipse.major_axis = *major_axis;
                ellipse.normal = *extrusion;
                ellipse.minor_axis_ratio = *axis_ratio;
                ellipse.start_parameter = *start_angle;
                ellipse.end_parameter = *end_angle;
                Some(EntityType::Ellipse(ellipse))
            }
            
            CadTemplate::LwPolyline { entity_data, flag, const_width, elevation, thickness, extrusion, vertices } => {
                let mut lwpoly = LwPolyline::new();
                lwpoly.common = self.build_entity_common(entity_data);
                lwpoly.is_closed = (*flag & 1) != 0;
                lwpoly.constant_width = *const_width;
                lwpoly.elevation = *elevation;
                lwpoly.thickness = *thickness;
                lwpoly.normal = *extrusion;
                
                for v in vertices {
                    let mut vertex = LwVertex::new(Vector2::new(v.point.x, v.point.y));
                    vertex.start_width = v.start_width;
                    vertex.end_width = v.end_width;
                    vertex.bulge = v.bulge;
                    lwpoly.vertices.push(vertex);
                }
                
                Some(EntityType::LwPolyline(lwpoly))
            }
            
            CadTemplate::Text { entity_data, insertion, alignment, extrusion, rotation, height, width_factor, value, horizontal_alignment, vertical_alignment, .. } => {
                let mut text = Text::new();
                text.common = self.build_entity_common(entity_data);
                text.insertion_point = *insertion;
                text.alignment_point = Some(*alignment);
                text.normal = *extrusion;
                text.rotation = *rotation;
                text.height = *height;
                text.width_factor = *width_factor;
                text.value = value.clone();
                
                text.horizontal_alignment = match *horizontal_alignment {
                    0 => crate::entities::text::TextHorizontalAlignment::Left,
                    1 => crate::entities::text::TextHorizontalAlignment::Center,
                    2 => crate::entities::text::TextHorizontalAlignment::Right,
                    3 => crate::entities::text::TextHorizontalAlignment::Aligned,
                    4 => crate::entities::text::TextHorizontalAlignment::Middle,
                    5 => crate::entities::text::TextHorizontalAlignment::Fit,
                    _ => crate::entities::text::TextHorizontalAlignment::Left,
                };
                text.vertical_alignment = match *vertical_alignment {
                    0 => crate::entities::text::TextVerticalAlignment::Baseline,
                    1 => crate::entities::text::TextVerticalAlignment::Bottom,
                    2 => crate::entities::text::TextVerticalAlignment::Middle,
                    3 => crate::entities::text::TextVerticalAlignment::Top,
                    _ => crate::entities::text::TextVerticalAlignment::Baseline,
                };
                Some(EntityType::Text(text))
            }
            
            CadTemplate::MText { entity_data, insertion, extrusion, rect_width, text_height, attachment, contents, .. } => {
                let mut mtext = MText::new();
                mtext.common = self.build_entity_common(entity_data);
                mtext.insertion_point = *insertion;
                mtext.normal = *extrusion;
                mtext.rectangle_width = *rect_width;
                mtext.height = *text_height;
                mtext.attachment_point = match *attachment {
                    1 => crate::entities::mtext::AttachmentPoint::TopLeft,
                    2 => crate::entities::mtext::AttachmentPoint::TopCenter,
                    3 => crate::entities::mtext::AttachmentPoint::TopRight,
                    4 => crate::entities::mtext::AttachmentPoint::MiddleLeft,
                    5 => crate::entities::mtext::AttachmentPoint::MiddleCenter,
                    6 => crate::entities::mtext::AttachmentPoint::MiddleRight,
                    7 => crate::entities::mtext::AttachmentPoint::BottomLeft,
                    8 => crate::entities::mtext::AttachmentPoint::BottomCenter,
                    9 => crate::entities::mtext::AttachmentPoint::BottomRight,
                    _ => crate::entities::mtext::AttachmentPoint::TopLeft,
                };
                mtext.value = contents.clone();
                Some(EntityType::MText(mtext))
            }
            
            CadTemplate::Spline { entity_data, degree, closed, periodic, rational, knots, weights, control_points, fit_points, .. } => {
                let mut spline = Spline::new();
                spline.common = self.build_entity_common(entity_data);
                spline.degree = *degree as i32;
                
                let mut spline_flags = SplineFlags::new();
                spline_flags.closed = *closed;
                spline_flags.periodic = *periodic;
                spline_flags.rational = *rational;
                spline.flags = spline_flags;
                
                spline.knots = knots.clone();
                spline.weights = weights.clone();
                spline.control_points = control_points.clone();
                spline.fit_points = fit_points.clone();
                
                Some(EntityType::Spline(spline))
            }
            
            CadTemplate::Insert { entity_data, insertion_point, scale, rotation, extrusion, .. } => {
                let mut insert = Insert::new(String::new(), *insertion_point);
                insert.common = self.build_entity_common(entity_data);
                insert.x_scale = scale.x;
                insert.y_scale = scale.y;
                insert.z_scale = scale.z;
                insert.rotation = *rotation;
                insert.normal = *extrusion;
                Some(EntityType::Insert(insert))
            }
            
            CadTemplate::Solid { entity_data, thickness, elevation, extrusion, corner1, corner2, corner3, corner4 } => {
                let c1 = Vector3::new(corner1.x, corner1.y, *elevation);
                let c2 = Vector3::new(corner2.x, corner2.y, *elevation);
                let c3 = Vector3::new(corner3.x, corner3.y, *elevation);
                let c4 = Vector3::new(corner4.x, corner4.y, *elevation);
                let mut solid = Solid::new(c1, c2, c3, c4);
                solid.common = self.build_entity_common(entity_data);
                solid.thickness = *thickness;
                solid.normal = *extrusion;
                Some(EntityType::Solid(solid))
            }
            
            CadTemplate::Ray { entity_data, point, vector } => {
                let mut ray = Ray::new(*point, *vector);
                ray.common = self.build_entity_common(entity_data);
                Some(EntityType::Ray(ray))
            }
            
            CadTemplate::XLine { entity_data, point, vector } => {
                let mut xline = XLine::new(*point, *vector);
                xline.common = self.build_entity_common(entity_data);
                Some(EntityType::XLine(xline))
            }
            
            // Complex entities and non-graphical objects don't produce entities yet
            // TODO: Implement Hatch, Face3D, AttributeDefinition, AttributeEntity
            _ => None,
        }
    }
}

impl Default for DwgTemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
