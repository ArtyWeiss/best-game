use std::{ffi::OsStr, io::Write, path::Path};

use shaderc::{Compiler, ShaderKind};

const SPV_EXT: &'static str = "spv";
const VERT_EXT: &'static str = "vert";
const FRAG_EXT: &'static str = "frag";

fn main() {
    let shaders_path = Path::new("assets/shaders");
    let compiled_path = Path::new("assets/compiled");
    remove_compiled_shaders(compiled_path);
    compile_shaders(shaders_path, compiled_path);
}

fn remove_compiled_shaders(path: &Path) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for path in entries {
            if let Ok(entry) = path {
                if entry.path().extension().is_some_and(|ext| ext == OsStr::new(SPV_EXT)) {
                    std::fs::remove_file(entry.path()).expect("Failed to remove file");
                }
            }
        }
    }
}

fn compile_shaders(src: &Path, dst: &Path) {
    let compiler = shaderc::Compiler::new().unwrap();
    match std::fs::create_dir_all(dst) {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e),
    }

    for path in std::fs::read_dir(src).expect("Failed to read dir") {
        if let Ok(entry) = path {
            if entry.path().extension().is_some_and(|ext| ext == OsStr::new(VERT_EXT)) {
                let src = format!("{}/{}", src.display(), entry.file_name().to_str().unwrap());
                let dst = format!(
                    "{}/{}.{}",
                    dst.display(),
                    entry.file_name().to_str().unwrap(),
                    SPV_EXT
                );
                compile_and_write(&compiler, &Path::new(&src), Path::new(&dst), ShaderKind::Vertex);
            }
            if entry.path().extension().is_some_and(|ext| ext == OsStr::new(FRAG_EXT)) {
                let src = format!("{}/{}", src.display(), entry.file_name().to_str().unwrap());
                let dst = format!(
                    "{}/{}.{}",
                    dst.display(),
                    entry.file_name().to_str().unwrap(),
                    SPV_EXT
                );
                compile_and_write(
                    &compiler,
                    &Path::new(&src),
                    &Path::new(&dst),
                    ShaderKind::Fragment,
                );
            }
        }
    }
}

fn compile_and_write(compiler: &Compiler, src: &Path, dst: &Path, kind: ShaderKind) {
    println!("Compiling {} -> {}", src.display(), dst.display());
    let source = std::fs::read_to_string(src).expect("Failed to read");
    let name = src.file_name().unwrap().to_str().unwrap();
    let artifact =
        compiler.compile_into_spirv(&source, kind, name, "main", None).expect("Compilation failed");
    let mut compiled = std::fs::File::create_new(dst).expect("Failed to create file");
    compiled.write_all(artifact.as_binary_u8()).expect("Failed to write");
}
