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
    Face3D, face3d::InvisibleEdgeFlags,
    hatch::{Hatch, HatchPattern, HatchPatternType, HatchStyleType, HatchPatternLine,
            BoundaryPath, BoundaryPathFlags, BoundaryEdge, LineEdge, CircularArcEdge,
            EllipticArcEdge, SplineEdge},
    dimension::{Dimension, DimensionBase, DimensionType, AttachmentPointType,
                DimensionAligned, DimensionLinear, DimensionRadius, DimensionDiameter,
                DimensionAngular3Pt, DimensionAngular2Ln, DimensionOrdinate},
    polyline::{Polyline2D, Vertex2D, PolylineFlags, VertexFlags, SmoothSurfaceType},
    polyline3d::{Polyline3D, Vertex3DPolyline, Polyline3DFlags},
    Viewport, viewport::{ViewportStatusFlags, ViewportRenderMode, GridFlags},
    attribute_definition::{AttributeDefinition, AttributeFlags, MTextFlag,
                           HorizontalAlignment, VerticalAlignment},
    AttributeEntity,
    Block, BlockEnd,
    leader::{Leader, LeaderPathType, LeaderCreationType, HooklineDirection},
    tolerance::Tolerance,
    mline::{MLine, MLineVertex, MLineSegment, MLineJustification, MLineFlags},
    shape::Shape,
    solid3d::{Solid3D, Region, Body},
};
use super::object_reader::{CadTemplate, DwgEntityData, DimCommonData, TextTemplateData};

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
            
            CadTemplate::Face3D { entity_data, has_no_flags, z_is_zero, corners, invisible_edge } => {
                let mut face = Face3D::new(corners[0], corners[1], corners[2], corners[3]);
                face.common = self.build_entity_common(entity_data);
                face.invisible_edges = InvisibleEdgeFlags::from_bits(*invisible_edge as u8);
                Some(EntityType::Face3D(face))
            }

            CadTemplate::Hatch { entity_data, elevation, extrusion, pattern_name, is_solid_fill,
                                  is_associative, pattern_type, pattern_angle, pattern_scale,
                                  pattern_double, num_seed_points, seed_points, boundary_paths,
                                  pattern_def_lines } => {
                let mut hatch = if *is_solid_fill {
                    Hatch::solid()
                } else {
                    let mut pat = HatchPattern::new(pattern_name.clone());
                    for def_line in pattern_def_lines {
                        pat.lines.push(HatchPatternLine {
                            angle: def_line.angle,
                            base_point: def_line.base_point,
                            offset: def_line.offset,
                            dash_lengths: def_line.dash_lengths.clone(),
                        });
                    }
                    Hatch::with_pattern(pat)
                };
                hatch.common = self.build_entity_common(entity_data);
                hatch.elevation = *elevation;
                hatch.normal = *extrusion;
                hatch.is_solid = *is_solid_fill;
                hatch.is_associative = *is_associative;
                hatch.pattern_type = match *pattern_type {
                    0 => HatchPatternType::UserDefined,
                    1 => HatchPatternType::Predefined,
                    2 => HatchPatternType::Custom,
                    _ => HatchPatternType::Predefined,
                };
                hatch.pattern_angle = *pattern_angle;
                hatch.pattern_scale = *pattern_scale;
                hatch.is_double = *pattern_double;
                hatch.seed_points = seed_points.clone();

                for bp in boundary_paths {
                    let mut path = BoundaryPath::with_flags(BoundaryPathFlags::from_bits(bp.flags));
                    for edge in &bp.edges {
                        let boundary_edge = match edge {
                            super::object_reader::HatchEdge::Line { start, end } => {
                                BoundaryEdge::Line(LineEdge { start: *start, end: *end })
                            }
                            super::object_reader::HatchEdge::CircularArc { center, radius, start_angle, end_angle, is_ccw } => {
                                BoundaryEdge::CircularArc(CircularArcEdge {
                                    center: *center, radius: *radius,
                                    start_angle: *start_angle, end_angle: *end_angle,
                                    counter_clockwise: *is_ccw,
                                })
                            }
                            super::object_reader::HatchEdge::EllipticArc { center, major_axis, minor_ratio, start_angle, end_angle, is_ccw } => {
                                BoundaryEdge::EllipticArc(EllipticArcEdge {
                                    center: *center, major_axis_endpoint: *major_axis,
                                    minor_axis_ratio: *minor_ratio,
                                    start_angle: *start_angle, end_angle: *end_angle,
                                    counter_clockwise: *is_ccw,
                                })
                            }
                            super::object_reader::HatchEdge::Spline { degree, rational, periodic, knots, control_points, weights, fit_data } => {
                                // control_points in HatchEdge are Vector2, SplineEdge needs Vector3 (x, y, weight)
                                let ctrl_pts: Vec<Vector3> = control_points.iter().enumerate().map(|(i, pt)| {
                                    let w = weights.get(i).copied().unwrap_or(1.0);
                                    Vector3::new(pt.x, pt.y, w)
                                }).collect();
                                BoundaryEdge::Spline(SplineEdge {
                                    degree: *degree,
                                    rational: *rational,
                                    periodic: *periodic,
                                    knots: knots.clone(),
                                    control_points: ctrl_pts,
                                    fit_points: fit_data.as_ref().map(|(_, _, pts)| pts.clone()).unwrap_or_default(),
                                    start_tangent: fit_data.as_ref().map(|(s, _, _)| *s).unwrap_or(Vector2::new(0.0, 0.0)),
                                    end_tangent: fit_data.as_ref().map(|(_, e, _)| *e).unwrap_or(Vector2::new(0.0, 0.0)),
                                })
                            }
                        };
                        path.edges.push(boundary_edge);
                    }
                    // Handle polyline boundary paths
                    if !bp.polyline_vertices.is_empty() {
                        let verts: Vec<Vector2> = bp.polyline_vertices.iter().map(|(v, _)| *v).collect();
                        let polyedge = crate::entities::hatch::PolylineEdge::new(verts, bp.polyline_closed);
                        path.edges.push(BoundaryEdge::Polyline(polyedge));
                    }
                    hatch.paths.push(path);
                }
                Some(EntityType::Hatch(hatch))
            }

            CadTemplate::DimAligned { entity_data, dim_common, def_point, xline1_pt, xline2_pt } => {
                let mut dim = DimensionAligned::new(*xline1_pt, *xline2_pt);
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.definition_point = *def_point;
                Some(EntityType::Dimension(Dimension::Aligned(dim)))
            }

            CadTemplate::DimLinear { entity_data, dim_common, def_point, xline1_pt, xline2_pt, rotation, oblique_angle } => {
                let mut dim = DimensionLinear::new(*xline1_pt, *xline2_pt);
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.definition_point = *def_point;
                dim.rotation = *rotation;
                dim.ext_line_rotation = *oblique_angle;
                Some(EntityType::Dimension(Dimension::Linear(dim)))
            }

            CadTemplate::DimRadius { entity_data, dim_common, def_point, leader_len } => {
                let center = dim_common.clone_ins_pt;
                let mut dim = DimensionRadius::new(
                    Vector3::new(center.x, center.y, dim_common.elevation),
                    *def_point,
                );
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.leader_length = *leader_len;
                Some(EntityType::Dimension(Dimension::Radius(dim)))
            }

            CadTemplate::DimDiameter { entity_data, dim_common, def_point, leader_len } => {
                let center = dim_common.clone_ins_pt;
                let mut dim = DimensionDiameter::new(
                    Vector3::new(center.x, center.y, dim_common.elevation),
                    *def_point,
                );
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.leader_length = *leader_len;
                Some(EntityType::Dimension(Dimension::Diameter(dim)))
            }

            CadTemplate::DimAngular3Pt { entity_data, dim_common, def_point, xline1_pt, xline2_pt, center_pt } => {
                let mut dim = DimensionAngular3Pt::new(*center_pt, *xline1_pt, *xline2_pt);
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.definition_point = *def_point;
                Some(EntityType::Dimension(Dimension::Angular3Pt(dim)))
            }

            CadTemplate::DimOrdinate { entity_data, dim_common, def_point, feature_pt, leader_pt, ordinate_type } => {
                let is_x = (*ordinate_type & 0x02) == 0; // bit 1 clear = X ordinate
                let mut dim = DimensionOrdinate::new(*feature_pt, *leader_pt, is_x);
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.definition_point = *def_point;
                Some(EntityType::Dimension(Dimension::Ordinate(dim)))
            }

            CadTemplate::Polyline2D { entity_data, flags, curve_type, start_width, end_width,
                                       thickness, elevation, extrusion, .. } => {
                let mut poly = Polyline2D::new();
                poly.common = self.build_entity_common(entity_data);
                poly.flags = PolylineFlags::from_bits(*flags as u16);
                poly.smooth_surface = match *curve_type {
                    5 => SmoothSurfaceType::QuadraticBSpline,
                    6 => SmoothSurfaceType::CubicBSpline,
                    8 => SmoothSurfaceType::Bezier,
                    _ => SmoothSurfaceType::None,
                };
                poly.start_width = *start_width;
                poly.end_width = *end_width;
                poly.thickness = *thickness;
                poly.elevation = *elevation;
                poly.normal = *extrusion;
                // Vertices will be resolved from handles later if needed
                Some(EntityType::Polyline2D(poly))
            }

            CadTemplate::Polyline3D { entity_data, flags, curve_type, .. } => {
                let mut poly = Polyline3D::new();
                poly.common = self.build_entity_common(entity_data);
                poly.flags = Polyline3DFlags::from_bits(*flags as i32);
                poly.smooth_type = crate::entities::polyline3d::SmoothSurfaceType::from_value(*curve_type as i16);
                Some(EntityType::Polyline3D(poly))
            }

            CadTemplate::Viewport { entity_data, center, width, height, view_target,
                                     view_direction, view_twist_angle, view_height, lens_length,
                                     front_clip, back_clip, snap_angle, view_center, snap_base,
                                     snap_spacing, grid_spacing, circle_sides, frozen_layer_handles } => {
                let mut vp = Viewport::with_size(*center, *width, *height);
                vp.common = self.build_entity_common(entity_data);
                vp.view_target = *view_target;
                vp.view_direction = *view_direction;
                vp.twist_angle = *view_twist_angle;
                vp.view_height = *view_height;
                vp.lens_length = *lens_length;
                vp.front_clip_z = *front_clip;
                vp.back_clip_z = *back_clip;
                vp.snap_angle = *snap_angle;
                vp.view_center = Vector3::new(view_center.x, view_center.y, 0.0);
                vp.snap_base = Vector3::new(snap_base.x, snap_base.y, 0.0);
                vp.snap_spacing = Vector3::new(snap_spacing.x, snap_spacing.y, 0.0);
                vp.grid_spacing = Vector3::new(grid_spacing.x, grid_spacing.y, 0.0);
                vp.circle_sides = *circle_sides as i16;
                vp.frozen_layers = frozen_layer_handles.iter().map(|h| Handle::new(*h)).collect();
                Some(EntityType::Viewport(vp))
            }

            CadTemplate::AttDef { entity_data, text_data, version, prompt, tag, flags, field_length, lock_position } => {
                let mut attdef = AttributeDefinition::new(
                    tag.clone(),
                    prompt.clone(),
                    text_data.value.clone(),
                );
                attdef.common = self.build_entity_common(entity_data);
                attdef.insertion_point = text_data.insertion;
                attdef.alignment_point = text_data.alignment;
                attdef.normal = text_data.extrusion;
                attdef.height = text_data.height;
                attdef.rotation = text_data.rotation;
                attdef.width_factor = text_data.width_factor;
                attdef.oblique_angle = text_data.oblique_angle;
                attdef.text_generation_flags = text_data.generation_flags;
                attdef.horizontal_alignment = match text_data.horizontal_alignment {
                    1 => HorizontalAlignment::Center,
                    2 => HorizontalAlignment::Right,
                    3 => HorizontalAlignment::Aligned,
                    4 => HorizontalAlignment::Middle,
                    5 => HorizontalAlignment::Fit,
                    _ => HorizontalAlignment::Left,
                };
                attdef.vertical_alignment = match text_data.vertical_alignment {
                    1 => VerticalAlignment::Bottom,
                    2 => VerticalAlignment::Middle,
                    3 => VerticalAlignment::Top,
                    _ => VerticalAlignment::Baseline,
                };
                attdef.flags = AttributeFlags::from_bits(*flags as i32);
                attdef.field_length = *field_length as i16;
                attdef.lock_position = *lock_position;
                Some(EntityType::AttributeDefinition(attdef))
            }

            CadTemplate::Attrib { entity_data, text_data, version, tag, flags, field_length, lock_position } => {
                let mut attrib = AttributeEntity::new(
                    tag.clone(),
                    text_data.value.clone(),
                );
                attrib.common = self.build_entity_common(entity_data);
                attrib.insertion_point = text_data.insertion;
                attrib.alignment_point = text_data.alignment;
                attrib.normal = text_data.extrusion;
                attrib.height = text_data.height;
                attrib.rotation = text_data.rotation;
                attrib.width_factor = text_data.width_factor;
                attrib.oblique_angle = text_data.oblique_angle;
                attrib.text_generation_flags = text_data.generation_flags;
                attrib.horizontal_alignment = match text_data.horizontal_alignment {
                    1 => HorizontalAlignment::Center,
                    2 => HorizontalAlignment::Right,
                    3 => HorizontalAlignment::Aligned,
                    4 => HorizontalAlignment::Middle,
                    5 => HorizontalAlignment::Fit,
                    _ => HorizontalAlignment::Left,
                };
                attrib.vertical_alignment = match text_data.vertical_alignment {
                    1 => VerticalAlignment::Bottom,
                    2 => VerticalAlignment::Middle,
                    3 => VerticalAlignment::Top,
                    _ => VerticalAlignment::Baseline,
                };
                attrib.flags = AttributeFlags::from_bits(*flags as i32);
                attrib.field_length = *field_length as i16;
                attrib.lock_position = *lock_position;
                Some(EntityType::AttributeEntity(attrib))
            }

            CadTemplate::Trace { entity_data, thickness, elevation, extrusion, corner1, corner2, corner3, corner4 } => {
                // Trace is geometrically identical to Solid
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

            CadTemplate::Block { entity_data, name } => {
                let mut block = Block::new(name.clone(), Vector3::new(0.0, 0.0, 0.0));
                block.common = self.build_entity_common(entity_data);
                Some(EntityType::Block(block))
            }

            CadTemplate::BlockEnd { entity_data } => {
                let mut block_end = BlockEnd::new();
                block_end.common = self.build_entity_common(entity_data);
                Some(EntityType::BlockEnd(block_end))
            }

            CadTemplate::DimAngular2Ln { entity_data, dim_common, dimension_arc, first_point, second_point, angle_vertex, definition_point } => {
                let mut dim = DimensionAngular2Ln::default();
                self.apply_dim_common(&mut dim.base, entity_data, dim_common);
                dim.dimension_arc = *dimension_arc;
                dim.first_point = *first_point;
                dim.second_point = *second_point;
                dim.angle_vertex = *angle_vertex;
                dim.definition_point = *definition_point;
                Some(EntityType::Dimension(Dimension::Angular2Ln(dim)))
            }

            CadTemplate::Leader { entity_data, annot_type, path_type, vertices, normal,
                                  horizontal_direction, block_offset, annotation_offset,
                                  text_height, text_width, hookline_on_xdir, arrowhead_on, .. } => {
                let mut leader = Leader::new();
                leader.common = self.build_entity_common(entity_data);
                leader.path_type = LeaderPathType::from_value(*path_type);
                leader.creation_type = LeaderCreationType::from_value(*annot_type);
                leader.arrow_enabled = *arrowhead_on;
                leader.hookline_enabled = *hookline_on_xdir;
                leader.hookline_direction = if *hookline_on_xdir { HooklineDirection::Same } else { HooklineDirection::Opposite };
                leader.text_height = *text_height;
                leader.text_width = *text_width;
                leader.vertices = vertices.clone();
                leader.normal = *normal;
                leader.horizontal_direction = *horizontal_direction;
                leader.block_offset = *block_offset;
                leader.annotation_offset = *annotation_offset;
                Some(EntityType::Leader(leader))
            }

            CadTemplate::Tolerance { entity_data, insertion_point, direction, normal, text,
                                     text_height, dimgap, .. } => {
                let mut tol = Tolerance::new();
                tol.common = self.build_entity_common(entity_data);
                tol.insertion_point = *insertion_point;
                tol.direction = *direction;
                tol.normal = *normal;
                tol.text = text.clone();
                tol.text_height = *text_height;
                tol.dimension_gap = *dimgap;
                Some(EntityType::Tolerance(tol))
            }

            CadTemplate::MLine { entity_data, scale_factor, justification, start_point,
                                 normal, open_closed, lines_in_style, vertices, .. } => {
                let mut ml = MLine::new();
                ml.common = self.build_entity_common(entity_data);
                ml.scale_factor = *scale_factor;
                ml.justification = MLineJustification::from(*justification as i16);
                ml.start_point = *start_point;
                ml.normal = *normal;
                ml.flags = MLineFlags::from_bits_truncate(*open_closed);
                ml.style_element_count = *lines_in_style as usize;
                ml.vertices = vertices.iter().map(|v| {
                    let mut mv = MLineVertex::new(v.position);
                    mv.direction = v.direction;
                    mv.miter = v.miter;
                    mv.segments = v.segments.iter().map(|s| {
                        let mut seg = MLineSegment::new();
                        seg.parameters = s.parameters.clone();
                        seg.area_fill_parameters = s.area_fill_params.clone();
                        seg
                    }).collect();
                    mv
                }).collect();
                Some(EntityType::MLine(ml))
            }

            CadTemplate::Shape { entity_data, insertion_point, size, rotation, relative_x_scale,
                                 oblique_angle, thickness, shape_index, normal, .. } => {
                let mut shape = Shape::new();
                shape.common = self.build_entity_common(entity_data);
                shape.insertion_point = *insertion_point;
                shape.size = *size;
                shape.rotation = *rotation;
                shape.relative_x_scale = *relative_x_scale;
                shape.oblique_angle = *oblique_angle;
                shape.thickness = *thickness;
                shape.shape_number = *shape_index as i32;
                shape.normal = *normal;
                Some(EntityType::Shape(shape))
            }

            CadTemplate::Region { entity_data, .. } => {
                let mut region = Region::new();
                region.common = self.build_entity_common(entity_data);
                Some(EntityType::Region(region))
            }

            CadTemplate::Solid3D { entity_data, .. } => {
                let mut solid = Solid3D::new();
                solid.common = self.build_entity_common(entity_data);
                Some(EntityType::Solid3D(solid))
            }

            CadTemplate::Body { entity_data, .. } => {
                let mut body = Body::new();
                body.common = self.build_entity_common(entity_data);
                Some(EntityType::Body(body))
            }

            CadTemplate::Seqend { .. } | CadTemplate::Vertex2D { .. } | CadTemplate::Vertex3D { .. } => {
                // Sequence end, vertex objects are internal — not standalone entities
                None
            }

            // Table objects and dictionary — not entity types
            CadTemplate::Layer { .. } | CadTemplate::LineType { .. } | CadTemplate::BlockRecord { .. }
            | CadTemplate::Dictionary { .. } | CadTemplate::TextStyle { .. }
            | CadTemplate::DimStyle { .. } | CadTemplate::Unknown { .. } => {
                None
            }
        }
    }

    /// Apply dimension common data to a DimensionBase
    fn apply_dim_common(&self, base: &mut DimensionBase, entity_data: &DwgEntityData, dim: &DimCommonData) {
        base.common = self.build_entity_common(entity_data);
        base.version = dim.version;
        base.normal = dim.extrusion;
        base.text_middle_point = Vector3::new(dim.text_midpoint.x, dim.text_midpoint.y, dim.elevation);
        base.text_rotation = dim.text_rotation;
        base.horizontal_direction = dim.horiz_dir;
        base.insertion_point = Vector3::new(dim.ins_scale.x, dim.ins_scale.y, dim.ins_scale.z);
        base.actual_measurement = dim.actual_measurement;
        base.user_text = if dim.user_text.is_empty() { None } else { Some(dim.user_text.clone()) };
        base.line_spacing_factor = dim.linespacing_factor;
        base.attachment_point = match dim.attachment_point {
            1 => AttachmentPointType::TopLeft,
            2 => AttachmentPointType::TopCenter,
            3 => AttachmentPointType::TopRight,
            4 => AttachmentPointType::MiddleLeft,
            5 => AttachmentPointType::MiddleCenter,
            6 => AttachmentPointType::MiddleRight,
            7 => AttachmentPointType::BottomLeft,
            8 => AttachmentPointType::BottomCenter,
            9 => AttachmentPointType::BottomRight,
            _ => AttachmentPointType::MiddleCenter,
        };
    }
}

impl Default for DwgTemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
