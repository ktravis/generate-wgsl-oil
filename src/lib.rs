#![doc = include_str!("../README.md")]

mod error;
mod exports;
mod files;
mod imports;
mod module;
mod source;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use naga_oil::compose::Composer;
use naga_to_tokenstream::{ModuleToTokens, ModuleToTokensConfig};
use quote::format_ident;
use syn::parse_quote;

use crate::{error::decompose_mangled_name, exports::Export, source::Sourcecode};

fn module_items(source: &Sourcecode, module: &naga::Module, module_name: String) -> Vec<syn::Item> {
    let mut items = Vec::new();

    // Convert to info about the module
    let mut structs_filter: HashSet<String> = source
        .exports()
        .iter()
        .map(|export| match export {
            Export::Struct { struct_name } => struct_name.clone(),
        })
        .collect();
    let type_overrides = module
        .types
        .iter()
        .filter_map(|(_, t)| {
            let original_name = t.name.clone()?;
            let (module, name) = decompose_mangled_name(&original_name)?;
            structs_filter.remove(&original_name);
            let module = format_ident!("{}", module);
            let name = format_ident!("{}", name);
            Some((
                original_name,
                parse_quote! { super :: super :: super :: #module :: types :: #name },
            ))
        })
        .collect();
    let mut module_items = module.to_items(ModuleToTokensConfig {
        structs_filter: Some(structs_filter),
        gen_bytemuck: cfg!(feature = "bytemuck"),
        gen_glam: cfg!(feature = "glam"),
        gen_encase: cfg!(feature = "encase"),
        gen_naga: cfg!(feature = "naga"),
        type_overrides,
        module_name,
    });
    items.append(&mut module_items);

    items
}

pub fn generate_from_entrypoints(paths: &[String]) -> String {
    let project_root = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("code generation depends on cargo"),
    );

    let mut composer = Composer::default().with_capabilities(naga::valid::Capabilities::all());
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let mut shader_defs = HashMap::new();
    if cfg!(debug_assertions) {
        shader_defs.insert(
            "__DEBUG".to_string(),
            naga_oil::compose::ShaderDefValue::Bool(true),
        );
    }

    let items = paths
        .iter()
        .map(|path| {
            let module_name = PathBuf::from(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let name = format_ident!("{}", &module_name);
            let mut sourcecode = Sourcecode::new(project_root.clone(), path).unwrap();

            println!("cargo:rerun-if-changed={}", path);
            for p in sourcecode.relative_dependents() {
                println!("cargo:rerun-if-changed={}", p.to_str().unwrap());
            }

            let module = match sourcecode.compose(&mut composer, shader_defs.clone()) {
                Ok(module) => module,
                Err(e) => {
                    return parse_quote! {
                        pub mod #name {
                            compile_error!(#e);
                        }
                    };
                }
            };
            validator
                .validate(&module)
                .expect("Shader module validation failed");
            let mod_items = module_items(&sourcecode, &module, module_name);
            parse_quote! {
                pub mod #name {
                    #(#mod_items)*
                }
            }
        })
        .collect();
    #[cfg(feature = "prettyplease")]
    {
        prettyplease::unparse(&syn::File {
            items,
            shebang: None,
            attrs: vec![],
        })
    }
    #[cfg(not(feature = "prettyplease"))]
    {
        let result = quote::quote! {
            #(#items)*
        };
        result.to_string()
    }
}
