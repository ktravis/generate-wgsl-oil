use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    path::PathBuf,
};

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue, ShaderLanguage,
};

use crate::{
    exports::{strip_exports, Export},
    files::AbsoluteWGSLFilePathBuf,
    imports::ImportOrder,
    module::Module,
};

/// Shader sourcecode generated from the token stream provided
pub(crate) struct Sourcecode {
    exports: HashSet<Export>,
    root_module: Module,
    project_root: PathBuf,
    dependents: Vec<AbsoluteWGSLFilePathBuf>,
}

impl Sourcecode {
    pub(crate) fn new(project_root: PathBuf, path: &str) -> Result<Self, String> {
        let source_path = project_root.join(path);
        if !source_path.is_file() {
            if source_path.exists() {
                return Err(format!(
                    "could not find import `{}`: `{}` exists but is not a file",
                    path,
                    source_path.display()
                ));
            }
            return Err(format!(
                "could not find import `{}`: `{}` does not exist",
                path,
                source_path.display()
            ));
        }
        assert!(source_path.is_absolute());
        if source_path.extension() != Some(OsStr::new("wgsl")) {
            return Err(format!(
                "file `{}` does not have the required `.wgsl` extension",
                path,
            ));
        };

        let source_path = AbsoluteWGSLFilePathBuf::new(source_path);

        // Calculate top level exports
        let root_src = std::fs::read_to_string(source_path.as_path()).expect("asserted was file");
        let (_, exports) = strip_exports(&root_src);

        Ok(Self {
            root_module: Module::from_path(source_path),
            project_root,
            exports,
            dependents: Vec::new(),
        })
    }

    /// Uses naga_oil to process includes
    pub(crate) fn compose(
        &mut self,
        composer: &mut Composer,
        shader_defs: HashMap<String, ShaderDefValue>,
    ) -> Result<naga::Module, String> {
        // Traverses the imports in each file, starting with the file given by this object, to give all of the files required
        // and the order in which they need to be processed.
        let import_order = match ImportOrder::calculate(&self.root_module, &self.project_root) {
            Ok(import_order) => import_order,
            Err(err) => {
                return Err(format!("{}", err));
            }
        };

        // Calculate names of imports
        let reduced_names = import_order.reduced_names();

        // Add imports in order to naga-oil
        for import in import_order.modules() {
            let path = import.path();
            self.dependents.push(path.clone());

            let source = import.processed_source(&reduced_names, &self.project_root);
            if source.contains("#define") {
                return Err(format!(
                    "imported shader file `{}` contained a `#define` statement \
                    - only top-level files may contain preprocessor definitions",
                    path.to_string_lossy().to_string(),
                ));
            }

            let desc = ComposableModuleDescriptor {
                source: &source,
                file_path: path.to_str().unwrap(),
                language: ShaderLanguage::Wgsl,
                as_name: Some(reduced_names[&import].clone()),
                additional_imports: &[],
                shader_defs: shader_defs.clone(),
            };
            if let Err(e) = composer.add_composable_module(desc) {
                return Err(crate::error::format_compose_error(e, &composer));
            }
        }

        // Add main module to link everything
        composer
            .make_naga_module(NagaModuleDescriptor {
                source: &self
                    .root_module
                    .processed_source(&reduced_names, &self.project_root),
                file_path: &self.root_module.path().to_string_lossy().to_string(),
                additional_imports: &[],
                shader_defs,
                shader_type: naga_oil::compose::ShaderType::Wgsl,
            })
            .map_err(|e| crate::error::format_compose_error(e, &composer))
    }

    pub(crate) fn exports(&self) -> &HashSet<Export> {
        &self.exports
    }

    pub(crate) fn relative_dependents(&self) -> Vec<PathBuf> {
        self.dependents
            .iter()
            .map(|f| f.strip_prefix(&self.project_root).unwrap().to_path_buf())
            .collect()
    }
}
