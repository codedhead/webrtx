mod utils;

use std::borrow::Borrow;
use std::cmp::max;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use glsl_lang::ast::NodeContent;
use glsl_lang::visitor::{HostMut, Visit, VisitorMut};
use glsl_lang::{ast, parse::DefaultParse, transpiler::glsl as glsl_transpiler};
// use web_sys::console::assert;
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumString};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
      #[allow(unused_unsafe)]
      unsafe {
        web_sys::console::log_1(&format!( $( $t )* ).into());
      }
    }
}

// TODO: inout?
#[allow(non_camel_case_types)]
#[wasm_bindgen]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, AsRefStr)]
pub enum GlGlobalVarirableAsParam {
    gl_PrimitiveID,            // int
    gl_InstanceID,             // int
    gl_InstanceCustomIndexEXT, // int
    gl_GeometryIndexEXT,       // int
    gl_WorldRayOriginEXT,      // vec3
    gl_WorldRayDirectionEXT,   // vec3
    gl_ObjectRayOriginEXT,     // vec3
    gl_ObjectRayDirectionEXT,  // vec3
    gl_RayTminEXT,             //float
    gl_RayTmaxEXT,             //float
    gl_IncomingRayFlagsEXT,    //uint
    gl_HitTEXT,                //float
    gl_HitKindEXT,             //uint
    gl_ObjectToWorldEXT,       //mat4x3
    gl_WorldToObjectEXT,       //mat4x3
    gl_WorldToObject3x4EXT,    //mat3x4
    gl_ObjectToWorld3x4EXT,    //mat3x4
}

fn gl_global_variable_type(v: &GlGlobalVarirableAsParam) -> ast::TypeSpecifierNonArrayData {
    match v {
        GlGlobalVarirableAsParam::gl_PrimitiveID
        | GlGlobalVarirableAsParam::gl_InstanceID
        | GlGlobalVarirableAsParam::gl_InstanceCustomIndexEXT
        | GlGlobalVarirableAsParam::gl_GeometryIndexEXT => ast::TypeSpecifierNonArrayData::Int,
        GlGlobalVarirableAsParam::gl_WorldRayOriginEXT
        | GlGlobalVarirableAsParam::gl_WorldRayDirectionEXT
        | GlGlobalVarirableAsParam::gl_ObjectRayOriginEXT
        | GlGlobalVarirableAsParam::gl_ObjectRayDirectionEXT => {
            ast::TypeSpecifierNonArrayData::Vec3
        }
        GlGlobalVarirableAsParam::gl_RayTminEXT
        | GlGlobalVarirableAsParam::gl_RayTmaxEXT
        | GlGlobalVarirableAsParam::gl_HitTEXT => ast::TypeSpecifierNonArrayData::Float,
        GlGlobalVarirableAsParam::gl_IncomingRayFlagsEXT
        | GlGlobalVarirableAsParam::gl_HitKindEXT => ast::TypeSpecifierNonArrayData::UInt,
        GlGlobalVarirableAsParam::gl_ObjectToWorldEXT
        | GlGlobalVarirableAsParam::gl_WorldToObjectEXT => ast::TypeSpecifierNonArrayData::Mat43,
        GlGlobalVarirableAsParam::gl_WorldToObject3x4EXT
        | GlGlobalVarirableAsParam::gl_ObjectToWorld3x4EXT => ast::TypeSpecifierNonArrayData::Mat34,
    }
}

#[repr(u32)]
enum PackGlslType {
    UInt,
    Vec2,
    Vec3,
    Vec4,
}

#[wasm_bindgen]
pub struct ProcessedShaderInfo {
    processed_shader: String,
    processed_entry_point_prototype: String,
    forward_type_declarations: String,
    // TODO: merge theses code blocks into one
    unpacking_code: String,
    invocation_code: String,
    packing_code: String,
    global_variables: Vec<u32>, // TODO: Vec<String>?
    // hit_attributes: Vec<u32>,   // [...[PackGlslType, array_dims_terminated_by_0]];
    pub hit_attributes_num_words: u32,
    need_shader_record_data: bool,
    // TODO: find the first unused set number
    pub max_bind_set_number: i32,
}

#[wasm_bindgen]
impl ProcessedShaderInfo {
    pub fn processed_shader(&self) -> String {
        self.processed_shader.clone()
    }

    pub fn processed_entry_point_prototype(&self) -> String {
        self.processed_entry_point_prototype.clone()
    }

    pub fn forward_type_declarations(&self) -> String {
        self.forward_type_declarations.clone()
    }

    pub fn invocation_code(&self) -> String {
        self.invocation_code.clone()
    }

    pub fn packing_code(&self) -> String {
        self.packing_code.clone()
    }

    pub fn unpacking_code(&self) -> String {
        self.unpacking_code.clone()
    }

    pub fn global_variables(&self) -> Box<[u32]> {
        self.global_variables.clone().into_boxed_slice()
    }

    // pub fn hit_attributes(&self) -> Box<[u32]> {
    //     self.hit_attributes.clone().into_boxed_slice()
    // }
}

const PARAM_HIT_ATTRIBUTES: &str = "_crt_hattrs";
const PARAM_SHADER_RECORD_WORD_OFFSET: &str = "_crt_sr_wd_offset";
const INOUT_PARAM_HIT_REPORT: &str = "_crt_hit_report";
const OUT_PARAM_POTENTIAL_HIT: &str = "_crt_potential_hit_t";

const SHADER_RECORD_EXT: &str = "shaderRecordEXT";
const SBT_BUFFER_NAME: &str = "_crt_sbt_buf";
const BLOCK_IDENTIFIER_STRUCT_PREFIX: &str = "_crt_struct_";
const GL_GLOBAL_VARIABLE_REPLACED_PREFIX: &str = "_crt_";
const ACCELERATION_STRUCTURE_BLOCK_NAME: &str = "_crt_AccelerationStructureEXT";

#[allow(non_upper_case_globals)]
const reportIntersectionEXT: &str = "reportIntersectionEXT";
#[allow(non_upper_case_globals)]
const INTERNAL_API_NAME_reportIntersectionEXT: &str = "_crt_reportIntersectionEXT";
#[allow(non_upper_case_globals)]
const INTERNAL_NAME_reportIntersectionEXT_impl_no_rahit: &str =
    "_crt_reportIntersectionEXT_impl_no_rahit";

fn ray_payload_canonical_name(location: u32) -> String {
    format!("_crt_ray_payload_loc_{}", location)
}

type ArrayDimensions = Vec<u32>;

// type name [];
#[derive(Debug)]
struct PackedVariable(
    // multiple vars may share the same type, e.g. uint a, b;
    Rc<ast::TypeSpecifierNonArrayData>,
    String,
    Option<ArrayDimensions>,
);

fn serialize_packed_variables(data: &Option<PackedData>) -> Vec<u32> {
    if data.is_none() {
        return vec![];
    }
    data.as_ref()
        .unwrap()
        .variables
        .iter()
        .flat_map(|var| {
            let typ = match var.0.as_ref() {
                ast::TypeSpecifierNonArrayData::UInt => PackGlslType::UInt,
                ast::TypeSpecifierNonArrayData::Vec2 => PackGlslType::Vec2,
                ast::TypeSpecifierNonArrayData::Vec3 => PackGlslType::Vec3,
                ast::TypeSpecifierNonArrayData::Vec4 => PackGlslType::Vec4,
                unsupported @ _ => panic!("unsupported glsl type for packing: {:?}", unsupported),
            };
            let mut res = vec![typ as u32];
            if var.2.is_some() {
                res.append(&mut var.2.as_ref().unwrap().clone());
            }
            res.push(0);
            res
        })
        .collect()
}

impl PackedVariable {
    fn num_words(&self) -> u32 {
        let dim = if self.2.is_none() {
            1
        } else {
            self.2.as_ref().unwrap().iter().fold(1, |acc, x| acc * x)
        };
        let typ = self.0.borrow();
        dim * match typ {
            ast::TypeSpecifierNonArrayData::Bool
            | ast::TypeSpecifierNonArrayData::Int
            | ast::TypeSpecifierNonArrayData::UInt
            | ast::TypeSpecifierNonArrayData::Float => 1,
            ast::TypeSpecifierNonArrayData::IVec2
            | ast::TypeSpecifierNonArrayData::UVec2
            | ast::TypeSpecifierNonArrayData::Vec2 => 2,
            ast::TypeSpecifierNonArrayData::IVec3
            | ast::TypeSpecifierNonArrayData::UVec3
            | ast::TypeSpecifierNonArrayData::Vec3 => 3,
            ast::TypeSpecifierNonArrayData::IVec4
            | ast::TypeSpecifierNonArrayData::UVec4
            | ast::TypeSpecifierNonArrayData::Vec4 => 4,
            ast::TypeSpecifierNonArrayData::Mat4 => 16,
            _ => panic!("unsupported type: {:?}", typ),
        }
    }
}

// Either hit atrributes or shader record data.
// This can be a single variable, or a simple block.
#[derive(Debug)]
struct PackedData {
    block_identifier: Option<(String, String)>, // (struct_name, identifier)
    variables: Vec<PackedVariable>,
}

impl PackedData {
    fn total_num_words(&self) -> u32 {
        return self
            .variables
            .iter()
            .map(|v| v.num_words())
            .fold(0, |a, b| a + b);
    }
}

#[derive(Debug, Clone)]
struct ShaderProcessError(String);

fn array_spec_to_array_size(
    spec: &Option<ast::ArraySpecifier>,
) -> Result<Option<ArrayDimensions>, ShaderProcessError> {
    if spec.is_none() {
        return Ok(None);
    }
    let mut sizes: ArrayDimensions = ArrayDimensions::new();
    for d in spec.as_ref().unwrap().dimensions.iter() {
        if let ast::ArraySpecifierDimensionData::ExplicitlySized(ref size_expr) = d.content {
            match size_expr.content {
                ast::ExprData::IntConst(i) => {
                    sizes.push(i as u32);
                }
                ast::ExprData::UIntConst(u) => {
                    sizes.push(u);
                }
                _ => {
                    return Err(ShaderProcessError(String::from(
                        "unsupported array index expression",
                    )));
                }
            }
        } else {
            return Err(ShaderProcessError(String::from(
                "unsupported unsized array dimension",
            )));
        }
    }
    Ok(Some(sizes))
}

fn array_size_to_array_spec(sizes: &Option<ArrayDimensions>) -> Option<ast::ArraySpecifier> {
    let sizes = sizes.as_ref()?;
    Some(
        ast::ArraySpecifierData {
            dimensions: sizes
                .iter()
                .map(|dim| {
                    ast::ArraySpecifierDimensionData::ExplicitlySized(Box::new(
                        ast::ExprData::IntConst(*dim as i32).into(),
                    ))
                    .into()
                })
                .collect(),
        }
        .into(),
    )
}

fn to_param_qualifier(usage: &ast::StorageQualifierData) -> Option<ast::TypeQualifier> {
    match usage {
        ast::StorageQualifierData::Out | ast::StorageQualifierData::InOut => Some(
            ast::TypeQualifierData {
                qualifiers: vec![ast::TypeQualifierSpecData::Storage(usage.clone().into()).into()]
                    .into(),
            }
            .into(),
        ),
        _ => None,
    }
}

fn try_get_bind_set_number_from_qualifier(tq: &ast::TypeQualifierData) -> i32 {
    for q in tq.qualifiers.iter() {
        if let ast::TypeQualifierSpecData::Layout(l) = &q.content {
            for id in l.ids.iter() {
                if let ast::LayoutQualifierSpecData::Identifier(ref name, ref val) = id.content {
                    if name.as_str() == "set" {
                        let set = match val.as_ref().unwrap().content {
                            ast::ExprData::IntConst(i) => i,
                            ast::ExprData::UIntConst(u) => u as i32,
                            _ => panic!("unsupported bind set number"),
                        };
                        return set;
                    }
                }
            }
        }
    }
    -1
}

#[derive(Debug)]
struct RchitAnalyzer<'a> {
    shader_stage: &'a str,
    entry_point_name: &'a str,
    new_entry_point_name: &'a str,
    referenced_gl_global_variables:
        Option<HashMap<GlGlobalVarirableAsParam, &'a ast::StorageQualifierData>>,
    shader_record_data: Option<PackedData>,
    hit_attributes: Option<PackedData>,
    ray_payload_identifier_to_location: HashMap<String, u32>,
    processed_entry_point_prototype: String,
    entry_point_invocation: String,
    forward_type_decls: Vec<ast::DeclarationData>,
    max_bind_set_number: i32,
}

enum RayPayloadDataType {
    InOut,
    In,
}

impl<'a> RchitAnalyzer<'a> {
    fn is_hit_attribute(qualifier: &ast::TypeQualifierData) -> bool {
        qualifier.qualifiers.iter().any(|ref q| match &q.content {
            ast::TypeQualifierSpecData::Storage(s) => match &s.content {
                ast::StorageQualifierData::HitAttributeEXT => true,
                _ => false,
            },
            _ => false,
        })
    }

    fn is_ray_payload(qualifier: &ast::TypeQualifierData) -> Option<(RayPayloadDataType, u32)> {
        let mut storage = None;
        let mut location = None;
        for q in qualifier.qualifiers.iter() {
            match &q.content {
                ast::TypeQualifierSpecData::Storage(ref s) => {
                    storage = Some(s);
                }
                ast::TypeQualifierSpecData::Layout(ref l) => {
                    for id in l.ids.iter() {
                        if let ast::LayoutQualifierSpecData::Identifier(ref name, ref val) =
                            id.content
                        {
                            if name.as_str() == "location" {
                                // must have value
                                location = match val.as_ref().unwrap().content {
                                    ast::ExprData::IntConst(i) => Some(i as u32),
                                    ast::ExprData::UIntConst(u) => Some(u),
                                    _ => None,
                                };
                                break;
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        match &storage?.content {
            ast::StorageQualifierData::RayPayloadEXT => {
                Some((RayPayloadDataType::InOut, location.unwrap()))
            }
            ast::StorageQualifierData::RayPayloadInEXT => {
                Some((RayPayloadDataType::In, location.unwrap()))
            }
            _ => None,
        }
    }

    fn block_to_packed_data_and_struct_decl(
        block: &ast::Block,
    ) -> Result<(PackedData, Option<ast::DeclarationData>), ShaderProcessError> {
        let variables = block
            .fields
            .iter()
            .flat_map(|field_vars| {
                let typ = Rc::new(field_vars.ty.ty.content.clone());
                field_vars.identifiers.iter().map(move |id| {
                    array_spec_to_array_size(&id.array_spec).and_then(|sizes| {
                        Ok(PackedVariable(
                            Rc::clone(&typ),
                            id.ident.as_str().to_owned(),
                            sizes,
                        ))
                    })
                })
            })
            .collect::<Result<Vec<PackedVariable>, ShaderProcessError>>()?;

        if block.identifier.is_none() {
            // No need to declare the struct too, all fields will be global
            return Ok((
                PackedData {
                    block_identifier: None,
                    variables,
                },
                None,
            ));
        }

        let name = block.identifier.as_ref().unwrap().ident.as_str().to_owned();
        let mut typ = String::from(BLOCK_IDENTIFIER_STRUCT_PREFIX);
        typ.push_str(name.as_str());
        Ok((
            PackedData {
                block_identifier: Some((typ.clone(), name)),
                variables,
            },
            Some(
                ast::DeclarationData::InitDeclaratorList(
                    ast::InitDeclaratorListData {
                        head: ast::SingleDeclarationData {
                            ty: ast::TypeSpecifierNonArrayData::Struct(
                                ast::StructSpecifierData {
                                    name: Some(ast::TypeNameData(ast::SmolStr::new(typ)).into()),
                                    fields: block.fields.clone(),
                                }
                                .into(),
                            )
                            .into_node(),
                            name: None,
                            array_specifier: None,
                            initializer: None,
                        }
                        .into(),
                        tail: Vec::new(),
                    }
                    .into(),
                )
                .into(),
            ),
        ))
    }

    // (keep?, Option<insert new decl>)
    fn analyze_and_keep_declaration(
        &mut self,
        decl: &mut ast::Declaration,
    ) -> (bool, Option<ast::DeclarationData>) {
        match decl.content {
            ast::DeclarationData::InitDeclaratorList(ref mut dlist) => {
                if let Some(qualifier) = &dlist.head.ty.qualifier {
                    self.max_bind_set_number = max(
                        self.max_bind_set_number,
                        try_get_bind_set_number_from_qualifier(qualifier),
                    );

                    if RchitAnalyzer::is_hit_attribute(&qualifier.content) {
                        assert!(self.hit_attributes.is_none());
                        assert!(dlist.tail.len() == 0 && dlist.head.name.is_some());
                        self.hit_attributes = Some(PackedData {
                            block_identifier: None,
                            variables: vec![PackedVariable(
                                Rc::new(dlist.head.ty.ty.ty.content.clone()),
                                dlist.head.name.as_ref().unwrap().as_str().to_owned(),
                                array_spec_to_array_size(&dlist.head.ty.ty.array_specifier)
                                    .unwrap(),
                            )],
                        });
                        return (false, None);
                    }

                    // mutate accelerationStructureEXT into block
                    if ast::TypeSpecifierNonArrayData::AccelerationStructureEXT
                        == dlist.head.ty.ty.ty.content
                    {
                        assert!(dlist.tail.len() == 0);
                        assert!(dlist.head.name.is_some());
                        decl.content = ast::DeclarationData::Block(
                            ast::BlockData {
                                qualifier: qualifier.clone(),
                                name: ACCELERATION_STRUCTURE_BLOCK_NAME.into_node(),
                                fields: vec![ast::StructFieldSpecifierData {
                                    qualifier: None,
                                    ty: ast::TypeSpecifierData::from(
                                        ast::TypeSpecifierNonArrayData::UVec2,
                                    )
                                    .into(),
                                    identifiers: vec![ast::ArrayedIdentifierData::from(
                                        dlist.head.name.as_ref().unwrap().as_str(),
                                    )
                                    .into()],
                                }
                                .into()],
                                identifier: None,
                            }
                            .into(),
                        );
                        return (true, None);
                    }

                    match RchitAnalyzer::is_ray_payload(&qualifier.content) {
                        None => (),
                        Some((RayPayloadDataType::In, loc)) => {
                            assert!(dlist.head.name.is_some());
                            self.ray_payload_identifier_to_location
                                .insert(dlist.head.name.as_ref().unwrap().to_string(), loc);
                            // remove the variable definition
                            return (false, None);
                        }
                        Some((RayPayloadDataType::InOut, loc)) => {
                            assert!(dlist.head.name.is_some());
                            self.ray_payload_identifier_to_location
                                .insert(dlist.head.name.as_ref().unwrap().to_string(), loc);
                            // update name
                            dlist.head.name.as_mut().unwrap().0 =
                                ast::SmolStr::new(ray_payload_canonical_name(loc));
                            // remove qualifiers
                            dlist.head.ty.qualifier = None;
                            return (true, None);
                        }
                    }
                }
            }
            ast::DeclarationData::Block(ref block) => {
                self.max_bind_set_number = max(
                    self.max_bind_set_number,
                    try_get_bind_set_number_from_qualifier(&block.qualifier),
                );
                let is_sr = block.qualifier.qualifiers.iter().any(|q| match &q.content {
                    ast::TypeQualifierSpecData::Layout(ref l) => {
                        l.ids.iter().any(|id| match &id.content {
                            ast::LayoutQualifierSpecData::Identifier(ref id, _) => {
                                id.as_str() == SHADER_RECORD_EXT
                            }
                            _ => false,
                        })
                    }
                    _ => false,
                });
                if is_sr {
                    assert!(self.shader_record_data.is_none());
                    // TODO: error handling instead of panic
                    let result =
                        RchitAnalyzer::block_to_packed_data_and_struct_decl(block).unwrap();
                    self.shader_record_data = Some(result.0);
                    // TODO: looks like we can directly set decl.content instead of returning a new one.
                    return (false, result.1);
                }

                let is_hit_attr = RchitAnalyzer::is_hit_attribute(&block.qualifier.content);
                if is_hit_attr {
                    assert!(self.hit_attributes.is_none());
                    let result =
                        RchitAnalyzer::block_to_packed_data_and_struct_decl(block).unwrap();
                    self.hit_attributes = Some(result.0);
                    if result.1.is_some() {
                        self.forward_type_decls.push(result.1.unwrap());
                    }
                    return (false, None);
                }
            }
            _ => (),
        };
        (true, None)
    }

    fn mutate_entry_function_params(&mut self, f: &mut ast::FunctionPrototype) {
        if f.name.as_str() != self.entry_point_name {
            return;
        }
        f.name = ast::IdentifierData(ast::SmolStr::new(self.new_entry_point_name)).into();
        assert!(f.parameters.len() == 0);
        //ast::StatementData
        if self.hit_attributes.is_some() {
            let packed_data = self.hit_attributes.as_ref().unwrap();
            let qualifier = if self.shader_stage == "rint" {
                to_param_qualifier(&ast::StorageQualifierData::Out)
            } else {
                None
            };
            if let Some((struct_type, ident)) = &packed_data.block_identifier {
                f.parameters.push(to_function_param(
                    &ident,
                    qualifier,
                    ast::TypeSpecifierNonArrayData::TypeName(
                        ast::TypeNameData(ast::SmolStr::new(struct_type.clone())).into(),
                    ),
                    None,
                ));
            } else {
                for hattr_var in packed_data.variables.iter() {
                    f.parameters.push(to_function_param_with_array_spec(
                        &hattr_var.1,
                        qualifier.clone(),
                        hattr_var.0.as_ref().clone(),
                        array_size_to_array_spec(&hattr_var.2),
                    ));
                }
            }
        }
        // always pass shader record offset, e.g. in interesection shader calling anyhit, shader record may be required in anyhit
        f.parameters.push(to_function_param(
            PARAM_SHADER_RECORD_WORD_OFFSET,
            None,
            ast::TypeSpecifierNonArrayData::UInt,
            None,
        ));
        // referenced_gl_global_variables must exist
        for gl_var in self.referenced_gl_global_variables.as_ref().unwrap().iter() {
            f.parameters.push(to_function_param(
                gl_var
                    .0
                    .as_ref()
                    .replace("gl_", GL_GLOBAL_VARIABLE_REPLACED_PREFIX)
                    .as_str(), // TODO: store _crt_ version to avoid repeated replacement
                to_param_qualifier(gl_var.1),
                gl_global_variable_type(gl_var.0),
                None,
            ));
        }

        if self.shader_stage == "rahit" {
            f.parameters.push(to_function_param(
                INOUT_PARAM_HIT_REPORT,
                to_param_qualifier(&ast::StorageQualifierData::InOut),
                ast::TypeSpecifierNonArrayData::UInt,
                None,
            ));
        }
        if self.shader_stage == "rint" {
            f.parameters.push(to_function_param(
                OUT_PARAM_POTENTIAL_HIT,
                to_param_qualifier(&ast::StorageQualifierData::Out),
                ast::TypeSpecifierNonArrayData::Float,
                None,
            ));
        }
    }
}

fn to_function_param_with_array_spec(
    name: &str,
    qualifier: Option<ast::TypeQualifier>,
    typ: ast::TypeSpecifierNonArrayData,
    array_spec: Option<ast::ArraySpecifier>,
) -> ast::FunctionParameterDeclaration {
    ast::FunctionParameterDeclarationData::Named(
        qualifier,
        ast::FunctionParameterDeclaratorData {
            ty: ast::TypeSpecifierData::from(typ).into(),
            ident: ast::ArrayedIdentifierData {
                ident: ast::IdentifierData(ast::SmolStr::new(name)).into(),
                array_spec: array_spec,
            }
            .into(),
        }
        .into(),
    )
    .into()
}

fn to_function_param(
    name: &str,
    qualifier: Option<ast::TypeQualifier>,
    typ: ast::TypeSpecifierNonArrayData,
    array_size: Option<ast::ExprData>,
) -> ast::FunctionParameterDeclaration {
    to_function_param_with_array_spec(
        name,
        qualifier,
        typ,
        if array_size.is_none() {
            None
        } else {
            Some(
                ast::ArraySpecifierData {
                    dimensions: vec![ast::ArraySpecifierDimensionData::ExplicitlySized(Box::new(
                        array_size.unwrap().into(),
                    ))
                    .into()],
                }
                .into(),
            )
        },
    )
}

fn glsl_type_name_and_size(typ: &ast::TypeSpecifierNonArrayData) -> (&'static str, u32) {
    match typ {
        ast::TypeSpecifierNonArrayData::UInt => ("UINT", 1),
        ast::TypeSpecifierNonArrayData::Float => ("FLOAT", 1),
        ast::TypeSpecifierNonArrayData::Vec2 => ("VEC2", 2),
        ast::TypeSpecifierNonArrayData::Vec3 => ("VEC3", 3),
        ast::TypeSpecifierNonArrayData::Vec4 => ("VEC4", 4),
        ast::TypeSpecifierNonArrayData::Mat4 => ("MAT4", 16),
        _ => panic!("unsupported type: {:?}", typ),
    }
}

fn var_initializer_list<F>(unpack_element: &mut F, array_dims: &[u32]) -> ast::InitializerData
where
    F: FnMut() -> ast::Expr,
{
    if array_dims.len() == 0 {
        return unpack_element().into();
    }
    let cur_dim = array_dims[0];
    let sub_dims = &array_dims[1..];
    return ast::InitializerData::List(
        (0..cur_dim)
            .map(|_| var_initializer_list(unpack_element, sub_dims).into())
            .collect(),
    );
}

fn block_initializer(data: &Option<PackedData>) -> Option<ast::Statement> {
    let (struct_type, block_identifier) = data.as_ref()?.block_identifier.as_ref()?;
    Some(
        ast::StatementData::declare_var(
            ast::TypeSpecifierNonArrayData::TypeName(
                ast::TypeNameData(ast::SmolStr::new(struct_type.clone())).into(),
            ),
            ast::IdentifierData(ast::SmolStr::new(block_identifier.clone())),
            None,
            Some(
                ast::InitializerData::List(
                    data.as_ref()
                        .unwrap()
                        .variables
                        .iter()
                        .map(|var| {
                            ast::InitializerData::from(ast::ExprData::variable(var.1.as_str()))
                                .into()
                        })
                        .collect(),
                )
                .into(),
            ),
        )
        .into(),
    )
}

fn declare_packed_variables_with_optional_unpacking(
    data: &PackedData,
    unpacked_values: Option<Vec<ast::InitializerData>>,
) -> Vec<ast::Statement> {
    if unpacked_values.is_some() {
        assert!(
            unpacked_values.as_ref().unwrap().len() == data.variables.len(),
            "variable values length not match"
        );
    }
    if let Some((ref struct_type, ref identifier)) = data.block_identifier {
        vec![ast::StatementData::declare_var(
            ast::TypeSpecifierNonArrayData::TypeName(
                ast::TypeNameData(ast::SmolStr::new(struct_type.clone())).into(),
            ),
            ast::IdentifierData(ast::SmolStr::new(identifier.clone())),
            None,
            if unpacked_values.is_none() {
                None
            } else {
                Some(
                    ast::InitializerData::List(
                        unpacked_values
                            .unwrap()
                            .into_iter()
                            .map(|val| val.into_node())
                            .collect(),
                    )
                    .into(),
                )
            },
        )
        .into()]
    } else if unpacked_values.is_none() {
        data.variables
            .iter()
            .map(|var| {
                ast::StatementData::declare_var(
                    var.0.as_ref().clone().into_node(),
                    ast::IdentifierData(ast::SmolStr::new(var.1.clone())),
                    array_size_to_array_spec(&var.2),
                    None,
                )
                .into()
            })
            .collect()
    } else {
        unpacked_values
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, val)| {
                let var = &data.variables[i];
                ast::StatementData::declare_var(
                    var.0.as_ref().clone().into_node(),
                    ast::IdentifierData(ast::SmolStr::new(var.1.clone())),
                    array_size_to_array_spec(&var.2),
                    Some(val.into()),
                )
                .into()
            })
            .collect()
    }
}

fn unpacking_code_block(
    data: &PackedData,
    buf_name: &'static str,
    buf_type_suffix: &'static str,
) -> Vec<ast::Statement> {
    let mut word_offset = 0;
    let arg_buf_name = ast::ExprData::Variable(buf_name.into_node());
    let mut unpack_element = |typ: &ast::TypeSpecifierNonArrayData| {
        let (type_name, num_words_for_single_element) = glsl_type_name_and_size(typ);
        let args = vec![
            arg_buf_name.clone().into(),
            ast::ExprData::UIntConst(word_offset).into(),
        ];
        word_offset += num_words_for_single_element;
        ast::ExprData::FunCall(
            ast::FunIdentifierData::ident({
                ast::IdentifierData(ast::SmolStr::new(format!(
                    "UNPACK_{}{}",
                    type_name, buf_type_suffix
                )))
            })
            .into(),
            args,
        )
        .into()
    };

    let unpacked_values = data
        .variables
        .iter()
        .map(|var| {
            var_initializer_list(
                &mut || unpack_element(var.0.as_ref()),
                &var.2.as_ref().unwrap_or(&vec![]),
            )
        })
        .collect();

    declare_packed_variables_with_optional_unpacking(data, Some(unpacked_values))
}

fn pack_variable<F>(
    pack_element: &mut F,
    indices: &mut Vec<u32>,
    array_dims: &[u32],
    result: &mut Vec<ast::Statement>,
) where
    F: FnMut(&[u32]) -> ast::Expr,
{
    if array_dims.len() == 0 {
        result.push(
            ast::StatementData::Expression(
                ast::ExprStatementData(Some(pack_element(indices))).into(),
            )
            .into(),
        );
        return;
    }
    let cur_dim = array_dims[0];
    let sub_dims = &array_dims[1..];
    for i in 0..cur_dim {
        indices.push(i);
        pack_variable(pack_element, indices, sub_dims, result);
        indices.pop();
    }
}

fn packing_code_block(
    data: &PackedData,
    buf_name: &'static str,
    buf_type_suffix: &str,
) -> Vec<ast::Statement> {
    let mut word_offset = 0;
    let arg_buf_name = ast::ExprData::Variable(buf_name.into_node());
    let mut block_identifier_dot = String::new();
    if let Some((_, ident)) = &data.block_identifier {
        block_identifier_dot.push_str(ident);
        block_identifier_dot.push('.');
    }
    let mut pack_element =
        |var_name: &str, indices: &[u32], typ: &ast::TypeSpecifierNonArrayData| {
            let (type_name, num_words_for_single_element) = glsl_type_name_and_size(typ);
            let args: Vec<ast::Expr> = vec![
                arg_buf_name.clone().into(),
                ast::ExprData::UIntConst(word_offset).into(),
                ast::ExprData::Variable(
                    ast::IdentifierData(ast::SmolStr::new(format!(
                        "{}{}{}",
                        block_identifier_dot,
                        var_name,
                        indices
                            .iter()
                            .map(|i| format!("[{}]", i))
                            .collect::<Vec<String>>()
                            .join("")
                    )))
                    .into(),
                )
                .into(),
            ];
            word_offset += num_words_for_single_element;
            ast::ExprData::FunCall(
                ast::FunIdentifierData::ident({
                    ast::IdentifierData(ast::SmolStr::new(format!(
                        "PACK_{}{}",
                        type_name, buf_type_suffix
                    )))
                })
                .into(),
                args,
            )
            .into()
        };

    let mut unpacked_values = vec![];
    for var in data.variables.iter() {
        pack_variable(
            &mut |indices| pack_element(&var.1, indices, var.0.as_ref()),
            &mut vec![],
            &var.2.as_ref().unwrap_or(&vec![]),
            &mut unpacked_values,
        )
    }
    unpacked_values
}

struct GlobalVariablesReferencesAnalyzer<'a> {
    referenced_gl_global_variables:
        HashMap<GlGlobalVarirableAsParam, &'a ast::StorageQualifierData>,
}

impl<'a> VisitorMut for GlobalVariablesReferencesAnalyzer<'a> {
    fn visit_identifier(&mut self, ident: &mut ast::Identifier) -> Visit {
        let gl_var = GlGlobalVarirableAsParam::from_str(ident.as_str());
        if !gl_var.is_ok() {
            return Visit::Parent;
        }

        let gl_var = gl_var.unwrap();
        self.referenced_gl_global_variables
            .insert(gl_var, &ast::StorageQualifierData::In);
        // mutate
        ident.0 = ast::SmolStr::new(ident.0.replace("gl_", GL_GLOBAL_VARIABLE_REPLACED_PREFIX));
        Visit::Parent
    }
}

struct RayPayloadVariableRenamer<'a> {
    ray_payload_identifier_to_location: &'a HashMap<String, u32>,
}

impl<'a> VisitorMut for RayPayloadVariableRenamer<'a> {
    fn visit_identifier(&mut self, ident: &mut ast::Identifier) -> Visit {
        let ray_payload_loc = self.ray_payload_identifier_to_location.get(ident.as_str());
        if ray_payload_loc.is_some() {
            ident.0 = ast::SmolStr::new(ray_payload_canonical_name(*ray_payload_loc.unwrap()));
        }
        Visit::Parent
    }
}

struct ReportIntersectionExpression {
    found_args: Option<Vec<ast::Expr>>,
}
struct ReportIntersectionImpl {
    expr_analyzer: ReportIntersectionExpression,
}

impl VisitorMut for ReportIntersectionExpression {
    fn visit_expr(&mut self, expr: &mut ast::Expr) -> Visit {
        if let ast::ExprData::FunCall(ref mut fident, args) = &mut expr.content {
            if let Some(ref mut ident) = fident.as_ident_mut() {
                if ident.as_str() == reportIntersectionEXT {
                    ident.content = INTERNAL_API_NAME_reportIntersectionEXT.into();
                    assert!(
                        self.found_args.is_none(),
                        "only support a single reportIntersectionEXT call in one statement"
                    );
                    // also remove the args of original invocation
                    self.found_args = Some(std::mem::replace(args, vec![]));
                }
            }
        }
        Visit::Parent
    }
}
impl ReportIntersectionImpl {
    fn visit_compound_statement(&mut self, comp: &mut ast::CompoundStatement) {
        let mut i = 0;
        while i < comp.statement_list.len() {
            if let Some(new_stmt_before) = self.visit_statement(&mut comp.statement_list[i]) {
                comp.statement_list.insert(i, new_stmt_before);
                i += 1;
            }
            i += 1;
        }
    }

    fn visit_statement(&mut self, stmt: &mut ast::Statement) -> Option<ast::Statement> {
        if let ast::StatementData::Compound(comp) = &mut stmt.content {
            self.visit_compound_statement(comp);
            return None;
        }
        match &mut stmt.content {
            ast::StatementData::Compound(comp) => {
                self.visit_compound_statement(comp);
                return None;
            }
            ast::StatementData::Selection(sel) => {
                match &mut sel.rest.content {
                    ast::SelectionRestStatementData::Statement(st) => {
                        // TODO: make this a compound stmt if new stmt is returned
                        assert!(self.visit_statement(st).is_none(), "unsupported single statement in if/else body calling reportIntersectionEXT");
                    }
                    ast::SelectionRestStatementData::Else(first, second) => {
                        // TODO: make this a compound stmt if new stmt is returned
                        assert!(self.visit_statement(first).is_none(), "unsupported single statement in if/else body calling reportIntersectionEXT");
                        assert!(self.visit_statement(second).is_none(), "unsupported single statement in if/else body calling reportIntersectionEXT");
                    }
                }
                self.expr_analyzer.found_args = None;
                sel.cond.visit_mut(&mut self.expr_analyzer);
            }
            ast::StatementData::Declaration(decl) => {
                self.expr_analyzer.found_args = None;
                decl.visit_mut(&mut self.expr_analyzer);
            }
            ast::StatementData::Expression(expr) => {
                self.expr_analyzer.found_args = None;
                expr.visit_mut(&mut self.expr_analyzer);
            }
            ast::StatementData::Switch(swt) => {
                self.expr_analyzer.found_args = None;
                swt.head.visit_mut(&mut self.expr_analyzer);
            }
            _ => {
                return None;
            }
        }
        let args = self.expr_analyzer.found_args.take()?;
        Some(
            ast::StatementData::Expression(
                ast::ExprStatementData(Some(
                    ast::ExprData::FunCall(
                        ast::FunIdentifierData::ident(
                            INTERNAL_NAME_reportIntersectionEXT_impl_no_rahit,
                        )
                        .into(),
                        args,
                    )
                    .into(),
                ))
                .into(),
            )
            .into(),
        )
    }
}

fn function_prototype_to_invocation(prototype: &ast::FunctionPrototypeData) -> ast::Statement {
    ast::StatementData::Expression(
        ast::ExprStatementData(Some(
            ast::ExprData::FunCall(
                ast::FunIdentifierData::ident(prototype.name.as_str()).into(),
                prototype
                    .parameters
                    .iter()
                    .map(|p| match &p.content {
                        ast::FunctionParameterDeclarationData::Named(_, param) => {
                            ast::ExprData::variable(param.ident.ident.as_str()).into()
                        }
                        _ => panic!("unsupported unnamed fn param to arg"),
                    })
                    .collect(),
            )
            .into(),
        ))
        .into(),
    )
    .into()
}

impl<'a> VisitorMut for RchitAnalyzer<'a> {
    fn visit_function_definition(&mut self, f: &mut ast::FunctionDefinition) -> Visit {
        {
            // only works for func def after ray_payload declaration
            let mut ray_payload_renamer = RayPayloadVariableRenamer {
                ray_payload_identifier_to_location: &self.ray_payload_identifier_to_location,
            };
            f.visit_mut(&mut ray_payload_renamer);
        }

        if f.prototype.name.as_str() != self.entry_point_name {
            return Visit::Parent;
        }

        {
            let mut gl_collector = GlobalVariablesReferencesAnalyzer {
                referenced_gl_global_variables: HashMap::new(),
            };
            f.visit_mut(&mut gl_collector);
            // TODO: sometimes references are from unexpanded macros
            if self.shader_stage == "rint" {
                gl_collector.referenced_gl_global_variables.insert(
                    GlGlobalVarirableAsParam::gl_RayTminEXT,
                    &ast::StorageQualifierData::In,
                );
                gl_collector.referenced_gl_global_variables.insert(
                    GlGlobalVarirableAsParam::gl_RayTmaxEXT,
                    &ast::StorageQualifierData::In,
                );
            }
            self.referenced_gl_global_variables = Some(gl_collector.referenced_gl_global_variables);
        }
        // TODO: this is stricter than glsl macro expansion
        // if self.shader_stage == "rint" {
        //     let mut report_intersection_impl = ReportIntersectionImpl {
        //         expr_analyzer: ReportIntersectionExpression { found_args: None },
        //     };
        //     report_intersection_impl.visit_compound_statement(&mut f.statement);
        // }

        self.mutate_entry_function_params(&mut f.prototype);
        // TODO: only unpack referenced variables
        if self.shader_record_data.is_some() {
            f.statement.statement_list.splice(
                0..0,
                unpacking_code_block(
                    self.shader_record_data.as_ref().unwrap(),
                    SBT_BUFFER_NAME,
                    "_FROM_SBT_BUFFER",
                )
                .into_iter(),
            );
        }
        // let it panic
        glsl_transpiler::show_function_prototype(
            &mut self.processed_entry_point_prototype,
            &f.prototype,
            &mut glsl_transpiler::FormattingState::default(), //??
        )
        .unwrap();
        glsl_transpiler::show_statement(
            &mut self.entry_point_invocation,
            &function_prototype_to_invocation(&f.prototype),
            &mut glsl_transpiler::FormattingState::default(), //??
        )
        .unwrap();
        Visit::Parent
    }

    fn visit_translation_unit(&mut self, unit: &mut ast::TranslationUnit) -> Visit {
        let mut i = 0;
        let ext_decls = &mut unit.0;
        // vec.retain does not work because it forces immutable reference of element
        while i < ext_decls.len() {
            match ext_decls[i].content {
                ast::ExternalDeclarationData::Declaration(ref mut decl) => {
                    match self.analyze_and_keep_declaration(decl) {
                        (true, None) => {}
                        (false, None) => {
                            ext_decls.remove(i);
                            continue;
                        }
                        (false, Some(new_decl)) => {
                            // replace
                            // ext_decls[i] = ast::ExternalDeclarationData::Declaration(new_decl.into()).into();
                            decl.content = new_decl;
                        }
                        _ => panic!("no way"),
                    }
                }
                ast::ExternalDeclarationData::Preprocessor(ref mut prep) => match prep.content {
                    ast::PreprocessorData::Version(_) => {
                        ext_decls.remove(i);
                        continue;
                    }
                    ast::PreprocessorData::Extension(ref ext) => {
                        if let ast::PreprocessorExtensionNameData::Specific(name) =
                            &ext.name.content
                        {
                            if name == "GL_EXT_ray_tracing" {
                                ext_decls.remove(i);
                                continue;
                            }
                        }
                    }
                    ast::PreprocessorData::Pragma(ref pragma) => {
                        if pragma.command.contains("shader_stage(") {
                            ext_decls.remove(i);
                            continue;
                        }
                    }
                    _ => (),
                },
                _ => {}
            }
            i += 1;
        }
        Visit::Children
    }
}

fn statements_to_string(statements: &Vec<ast::Statement>) -> Result<String, JsValue> {
    let mut buffer = String::new();
    for stmt in statements.iter() {
        match glsl_transpiler::show_statement(
            &mut buffer,
            &stmt,
            &mut glsl_transpiler::FormattingState::default(), //??
        ) {
            Err(error) => return Err(JsValue::from(format!("{}", error))),
            _ => (),
        }
    }
    Ok(buffer)
}

#[wasm_bindgen]
pub fn process(
    code: &str,
    shader_stage: &str,
    entry_point_name: &str,
    new_entry_point_name: &str,
) -> Result<ProcessedShaderInfo, JsValue> {
    utils::set_panic_hook();
    log!("start parsing...");
    let mut ast = match ast::TranslationUnit::parse_with_options(
        code,
        &glsl_lang::parse::ParseOptions {
            default_version: 460, // TODO: or 450?
            target_vulkan: true,
            allow_rs_ident: false,
            ..Default::default()
        }
        .into(),
    ) {
        Ok(ast) => ast.0,
        Err(error) => return Err(JsValue::from(format!("{}", error))),
    };

    let mut analyzer = RchitAnalyzer {
        shader_stage,
        entry_point_name,
        new_entry_point_name,
        shader_record_data: None,
        hit_attributes: None,
        referenced_gl_global_variables: None,
        ray_payload_identifier_to_location: HashMap::new(),
        processed_entry_point_prototype: String::new(),
        entry_point_invocation: String::new(),
        forward_type_decls: vec![],
        max_bind_set_number: -1,
    };
    ast.visit_mut(&mut analyzer);
    // log!("analysis: {:?}", analyzer);

    // log!("parsed ast result: {:?}", ast);
    let mut buffer = String::new();
    match glsl_transpiler::show_translation_unit(
        &mut buffer,
        &ast,
        glsl_transpiler::FormattingState::default(),
    ) {
        Err(error) => return Err(JsValue::from(format!("{}", error))),
        _ => (),
    };
    // log!("serialize result: {:?}", serialize_result);
    // log!("output: {}", buffer);

    log!("done parsing.");
    Ok(ProcessedShaderInfo {
        processed_shader: buffer,
        processed_entry_point_prototype: analyzer.processed_entry_point_prototype,
        forward_type_declarations: statements_to_string(
            &analyzer
                .forward_type_decls
                .iter()
                // TODO: move out and no clone
                .map(|d| ast::StatementData::Declaration(d.clone().into()).into())
                .collect(),
        )?,
        invocation_code: analyzer.entry_point_invocation,
        unpacking_code: {
            if analyzer.hit_attributes.is_none() {
                String::from("")
            } else if shader_stage == "rint" {
                // just declare variables
                statements_to_string(&declare_packed_variables_with_optional_unpacking(
                    analyzer.hit_attributes.as_ref().unwrap(),
                    None,
                ))?
            } else {
                statements_to_string(&unpacking_code_block(
                    analyzer.hit_attributes.as_ref().unwrap(),
                    PARAM_HIT_ATTRIBUTES,
                    "_FROM_FLOAT_BUFFER",
                ))?
            }
        },
        packing_code: if shader_stage == "rint" {
            statements_to_string(&packing_code_block(
                analyzer.hit_attributes.as_ref().unwrap(),
                PARAM_HIT_ATTRIBUTES,
                "_INTO_FLOAT_BUFFER",
            ))?
        } else {
            String::from("")
        },
        need_shader_record_data: analyzer.shader_record_data.is_some(),
        // hit_attributes: serialize_packed_variables(&analyzer.hit_attributes),
        hit_attributes_num_words: match analyzer.hit_attributes {
            None => 0,
            Some(ref data) => data.total_num_words(),
        },
        max_bind_set_number: analyzer.max_bind_set_number,
        global_variables: analyzer
            .referenced_gl_global_variables
            .as_ref()
            .unwrap()
            .iter()
            .map(|var| *var.0 as u32)
            .collect(),
    })
}
