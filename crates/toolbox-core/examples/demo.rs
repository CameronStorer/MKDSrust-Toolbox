//! Rebuild the browser demo from generated fixtures only: cargo run -p toolbox-core --example demo.
use toolbox_core::{
    project::{Manifest, Project},
    service::{Request, Service},
    session::RomSession,
    synthetic,
};
fn main() -> toolbox_core::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = root.join(".cache/demo-project-v1");
    std::fs::create_dir_all(&folder)?;
    let session = RomSession::from_bytes(
        synthetic::rom(),
        std::path::PathBuf::from("/user/roms/synthetic.nds"),
    )?;
    let mut project = if folder.join("project.toml").exists() {
        Project::open(&folder, &session)?
    } else {
        Project::create(&folder,&session,Manifest{id:"studio.colour-study".into(),name:"Colour study".into(),author:"Studio".into(),description:"A small experiment in colour and character. Built entirely from synthetic fixtures.".into(),..Manifest::default()})?
    };
    let path = "data/Studio/demo.carc/art/studio.nclr";
    let mut p = nitro_formats::graphics::Nclr::parse(session.file(path)?)?;
    p.colors[9] = 0x5b1a;
    project.edit(path, p.write(), "Shift palette toward lavender", &session)?;
    let mut service = Service {
        session: Some(session),
        project: Some(project),
        persist_settings: false,
        ..Service::default()
    };
    service.dispatch(Request::Select { path: path.into() })?;
    let mut snapshot = service.snapshot()?;
    snapshot.project_path = "/projects/colour-study".into();
    snapshot.rom_path = "/user/roms/synthetic.nds".into();
    let out = root.join("app/src/demo.json");
    std::fs::write(out, serde_json::to_vec_pretty(&snapshot)?)?;
    let session = service.session.as_ref().unwrap();
    let messages = session
        .nodes
        .iter()
        .filter(|n| n.format == "bmg")
        .map(|n| {
            Ok((
                n.path.clone(),
                toolbox_core::preview::preview(session.file(&n.path)?)?.messages,
            ))
        })
        .collect::<toolbox_core::Result<std::collections::BTreeMap<_, _>>>()?;
    let tiles = nitro_formats::graphics::Ncgr::parse(
        session.file("data/Studio/demo.carc/art/studio.ncgr")?,
    )?;
    let pixels = (0..128 * 128)
        .map(|i| {
            let (x, y) = (i % 128, i / 128);
            tiles.pixels[(y / 8 * 16 + x / 8) * 64 + y % 8 * 8 + x % 8]
        })
        .collect::<Vec<_>>();
    let hex = session
        .nodes
        .iter()
        .map(|n| Ok((n.path.clone(), session.file(&n.path)?.to_vec())))
        .collect::<toolbox_core::Result<std::collections::BTreeMap<_, _>>>()?;
    std::fs::write(
        root.join("app/src/demo-assets.json"),
        serde_json::to_vec(&serde_json::json!({
            "messages": messages, "pixels":pixels, "width":128, "height":128, "hex":hex,
        }))?,
    )?;
    let texture = toolbox_core::preview::image(
        service.project.as_ref(),
        session,
        "data/Studio/texture.nsbtx",
        None,
        None,
        0,
        0,
    )?;
    std::fs::write(
        root.join("app/public/demo/texture.png"),
        toolbox_core::preview::png(&texture)?,
    )?;
    let image = toolbox_core::preview::image(
        service.project.as_ref(),
        service.session.as_ref().unwrap(),
        "data/Studio/demo.carc/art/studio.ncgr",
        Some(path),
        Some("data/Studio/demo.carc/art/studio.nscr"),
        0,
        0,
    )?;
    std::fs::create_dir_all(root.join("app/public/demo"))?;
    std::fs::write(
        root.join("app/public/demo/studio.png"),
        toolbox_core::preview::png(&image)?,
    )?;
    // The application mark is drawn here so no platform icon generator is required.
    let mut rgba = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let on = (7..25).contains(&x) && (8..12).contains(&y)
                || (14..18).contains(&x) && (12..25).contains(&y);
            let c = if on {
                [166, 221, 151, 255]
            } else {
                [24, 28, 31, 255]
            };
            let at = (y * 32 + x) * 4;
            rgba[at..at + 4].copy_from_slice(&c);
        }
    }
    let icon = nitro_formats::graphics::Image {
        width: 32,
        height: 32,
        rgba,
    };
    std::fs::write(
        root.join("app/src-tauri/icons/icon.png"),
        toolbox_core::preview::png(&icon)?,
    )?;
    if std::env::args().any(|arg| arg == "--pack") {
        let (bytes, warnings) = toolbox_core::pack::build(
            service.project.as_ref().unwrap(),
            service.session.as_ref().unwrap(),
        )?;
        let output = root.join(".cache/demo-mod.zip");
        std::fs::write(&output, bytes)?;
        println!("{}", output.display());
        for warning in warnings {
            eprintln!("{warning}");
        }
    }
    Ok(())
}
