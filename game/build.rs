use std::{ffi::OsStr, io::Write, path::Path};

use shaderc::{Compiler, ShaderKind};

const SPV_EXT: &'static str = "spv";
const VERT_EXT: &'static str = "vert";
const FRAG_EXT: &'static str = "frag";

fn main() {
    let shaders_path = Path::new("assets/shaders");
    remove_compiled_shaders(shaders_path);
    compile_shaders(shaders_path);
}

fn remove_compiled_shaders(path: &Path) {
    for path in std::fs::read_dir(path).unwrap() {
        if let Ok(entry) = path {
            if entry.path().extension().is_some_and(|ext| ext == OsStr::new(SPV_EXT)) {
                std::fs::remove_file(entry.path()).expect("Failed to remove file");
            }
        }
    }
}

fn compile_shaders(path: &Path) {
    let compiler = shaderc::Compiler::new().unwrap();
    for path in std::fs::read_dir(path).expect("Failed to read dir") {
        if let Ok(entry) = path {
            if entry.path().extension().is_some_and(|ext| ext == OsStr::new(VERT_EXT)) {
                compile_and_write(&compiler, &entry.path(), ShaderKind::Vertex);
            }
            if entry.path().extension().is_some_and(|ext| ext == OsStr::new(FRAG_EXT)) {
                compile_and_write(&compiler, &entry.path(), ShaderKind::Fragment);
            }
        }
    }
}

fn compile_and_write(compiler: &Compiler, path: &Path, kind: ShaderKind) {
    let source = std::fs::read_to_string(path).expect("Failed to read");
    let artifact = compiler
        .compile_into_spirv(&source, kind, path.to_str().unwrap(), "main", None)
        .expect("Compilation failed");

    let compiled_name = format!("{}.{}", path.display(), SPV_EXT);
    let compiled_path = Path::new(&compiled_name);
    println!("{}", compiled_path.display());
    let mut compiled = std::fs::File::create_new(compiled_path).expect("Failed to create file");
    compiled.write_all(artifact.as_binary_u8()).expect("Failed to write");
}
