use std::path::{Path, PathBuf};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum IncludeType {
    Relative,
    Standard,
}

impl From<shaderc::IncludeType> for IncludeType {
    fn from(include_type: shaderc::IncludeType) -> Self {
        match include_type {
            shaderc::IncludeType::Relative => IncludeType::Relative,
            shaderc::IncludeType::Standard => IncludeType::Standard,
        }
    }
}

pub struct ResolvedInclude {
    pub resolved_path: PathBuf,
    pub content: String,
}

impl Into<shaderc::ResolvedInclude> for ResolvedInclude {
    fn into(self) -> shaderc::ResolvedInclude {
        shaderc::ResolvedInclude {
            content: self.content,
            resolved_name: self.resolved_path.to_str().unwrap().to_string(),
        }
    }
}

pub(crate) fn include_impl(
    requested_path: &Path,
    include_type: IncludeType,
    requested_from: &Path,
    include_depth: usize,
) -> Result<ResolvedInclude, String> {
    log::trace!(
        "include file {:?} {:?} {:?} {:?}",
        requested_path,
        include_type,
        requested_from,
        include_depth
    );

    let resolved_path = match include_type {
        IncludeType::Relative => {
            if requested_path.is_absolute() {
                let path = requested_path.to_path_buf();
                log::trace!("absolute path {:?}", path);
                path
            } else {
                let path = requested_from.parent().unwrap().join(requested_path);
                log::trace!("from: {:?} relative path: {:?}", requested_from, path);
                path
            }
        }
        IncludeType::Standard => {
            //TODO: Implement include paths
            requested_from.parent().unwrap().join(requested_path)
        }
    };

    let content = std::fs::read_to_string(&resolved_path).map_err(|e| {
        format!(
            "Could not read file {:?} when trying to include {:?} from {:?}: {:?}",
            resolved_path, requested_path, requested_from, e
        )
    })?;

    Ok(ResolvedInclude {
        resolved_path,
        content,
    })
}

pub(crate) fn shaderc_include_callback(
    requested_path: &str,
    include_type: shaderc::IncludeType,
    requested_from: &str,
    include_depth: usize,
) -> shaderc::IncludeCallbackResult {
    let requested_path: PathBuf = requested_path.into();
    let requested_from: PathBuf = requested_from.into();
    include_impl(
        &requested_path,
        include_type.into(),
        &requested_from,
        include_depth,
    )
    .map(|x| x.into())
    .map_err(|x| x.into())
}
