use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/tsconfig.json");

    let web_dir = std::path::Path::new("web");

    if !web_dir.join("node_modules").exists() {
        let status = Command::new("pnpm")
            .args(["install", "--frozen-lockfile"])
            .current_dir(web_dir)
            .status()
            .expect("failed to run pnpm install — is pnpm installed?");
        assert!(status.success(), "pnpm install failed");
    }

    let status = Command::new("pnpm")
        .args(["run", "build"])
        .current_dir(web_dir)
        .status()
        .expect("failed to run pnpm run build — is pnpm installed?");
    assert!(status.success(), "pnpm run build failed");
}
