use app_lib::search::rrf_fuse;

#[test]
fn rrf_promotes_chunks_seen_in_both_lists() {
    let fused = rrf_fuse(&[1, 2], &[3, 1], 3);

    assert_eq!(fused.len(), 3);
    assert_eq!(fused[0].0, 1);
    assert!(fused[0].1 > fused[1].1);
    assert_eq!(fused[1].0, 3);
    assert_eq!(fused[2].0, 2);
}
