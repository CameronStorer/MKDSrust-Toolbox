use nitro_formats::{
    cell::{Nanr, Ncer},
    graphics::{Ncgr, Nclr},
    synthetic,
};
#[test]
fn cells_and_sequences_roundtrip_and_compose() {
    let c = synthetic::ncer();
    let a = synthetic::nanr();
    let cells = Ncer::parse(&c).unwrap();
    let anim = Nanr::parse(&a).unwrap();
    assert_eq!(cells.write(), c);
    assert_eq!(anim.write(), a);
    assert_eq!(anim.sequences[0].frames[1].cell, 1);
    let tiles = Ncgr::new(16, 1, 4, false, vec![1; 1024]).unwrap();
    let pal = Nclr::new(&[31; 256], true, false);
    let image = cells.render(0, &tiles, &pal).unwrap();
    assert_eq!(image.rgba.chunks_exact(4).filter(|c| c[3] != 0).count(), 64);
    for i in 0..c.len() {
        assert!(Ncer::parse(&c[..i]).is_err());
    }
    for i in 0..a.len() {
        assert!(Nanr::parse(&a[..i]).is_err());
    }
}
