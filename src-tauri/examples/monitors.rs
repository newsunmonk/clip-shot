// 모니터 좌표/스케일을 출력해 스티치/영역 캡처 좌표 가정을 검증한다.
// 실행: cargo run --example monitors
use xcap::Monitor;

fn main() {
    let monitors = Monitor::all().expect("monitors");
    println!("monitor count = {}", monitors.len());
    for m in &monitors {
        println!(
            "id={:?} name={:?} primary={:?} x={:?} y={:?} w={:?} h={:?} scale={:?}",
            m.id(),
            m.name(),
            m.is_primary(),
            m.x(),
            m.y(),
            m.width(),
            m.height(),
            m.scale_factor(),
        );
    }
}
