# parallel-image-cropper (picrop)

> 대량의 이미지 파일에서 특정 관심 영역(ROI)을 고속으로 일괄 추출하는 Rust 기반 멀티스레드 CLI 유틸리티

대규모 데이터셋 구축, 스캔 문서 규격화, UI 컴포넌트 캡처본 일괄 처리 등 반복적인 이미지 크롭 작업을 빠르게 처리하기 위해 개발된 고성능 도구입니다. Rust의 메모리 안전성과 Rayon 라이브러리를 통한 데이터 병렬 처리를 결합하여 단일 스레드 작업 대비 처리 시간을 획기적으로 단축했습니다.

---

## 주요 기능

- **멀티스레드 병렬 처리**: `rayon`의 작업 훔치기(Work-stealing) 풀을 활용해 시스템 CPU 코어 수에 맞춰 이미지 디코딩, 자르기, 인코딩 파이프라인을 병렬로 수행합니다.
- **방어적 좌표 계산(Clamping & Safe Subtraction)**:
- 시작점과 끝점 좌표의 순서(`x1 > x2` 등)에 구애받지 않고 유효 영역을 자동 계산합니다.
- 원본 이미지 해상도를 초과하는 좌표 입력 시 이미지 경계 내로 안전하게 클램핑(`min`)하며, `saturating_sub`를 통해 정수 언더플로우를 방지합니다.

- **락-프리(Lock-Free) 진행률 집계**: 뮤텍스(Mutex) 오버헤드 없이 `AtomicUsize` 기반 원자적 연산을 사용하여 멀티스레드 환경에서 처리 성공/실패 카운트를 추적합니다.
- **지원 포맷 자동 감지**: 확장자 기반의 이미지 포맷 유효성 검증을 거쳐 비이미지 파일을 안전하게 건너뜁니다.

---

## 기술 스택

- **Language**: Rust (Edition 2021)
- **Concurrency**: `rayon`
- **Image Processing**: `image` crate

---

## 핵심 엔지니어링 포인트

### 1. Work-Stealing 기반 대량 I/O 및 연산 최적화

단순 반복문(Single-thread)으로 고해상도 이미지를 순차 처리할 때 발생하는 디코딩/인코딩 CPU 병목을 해결하기 위해 `par_iter()`를 적용했습니다. 코어 수에 맞게 부하를 분산하여 수백 장 규모의 일괄 작업에서 선형적인 성능 향상을 도출합니다.

### 2. 안전한 기하 영역 계산 로직

```rust
let start_x = min(x, z).min(img_w);
let start_y = min(y, w).min(img_h);
let end_x = max(x, z).min(img_w);
let end_y = max(y, w).min(img_h);
let crop_w = end_x.saturating_sub(start_x);
let crop_h = end_y.saturating_sub(start_y);

```

사용자가 전달하는 좌표의 대소 관계가 뒤바뀌거나(`x1 > x2`), 이미지 해상도 바깥의 좌표를 전달하더라도 프로그램이 패닉(Panic)하지 않고 유효한 교집합 영역만 정확히 크롭하도록 설계했습니다.

---

## 빌드 및 실행 방법

### 요구 사항

- Rust 1.70 이상 및 Cargo

### 설치 ///임시

```bash
git clone https://github.com/your-username/parallel-image-cropper.git
cd parallel-image-cropper
cargo build --release

```

### 실행 ///임시

```bash
# 기본 사용법
# cargo run --release -- <x1> <y1> <x2> <y2> [입력_디렉토리] [출력_디렉토리]

# 현재 디렉토리의 이미지를 (100, 100)부터 (800, 1200)까지 잘라 ./cropped에 저장
cargo run --release -- 100 100 800 1200

# 특정 경로 지정
cargo run --release -- 0 50 1920 1030 ./raw_captures ./processed_dataset

```

### 매개변수 가이드

| 인자            | 필수 여부 | 설명                                                        |
| --------------- | --------- | ----------------------------------------------------------- |
| `x1`, `y1`      | **필수**  | 크롭 기준 사각형의 첫 번째 꼭짓점 좌표                      |
| `x2`, `y2`      | **필수**  | 크롭 기준 사각형의 대각선 꼭짓점 좌표                       |
| `입력_디렉토리` | 선택      | 처리할 대상 이미지가 위치한 폴더 (기본값: 현재 디렉토리)    |
| `출력_디렉토리` | 선택      | 크롭 결과물이 저장될 폴더 (기본값: `입력_디렉토리/cropped`) |

---

## 처리 파이프라인

```text
[입력 폴더 스캔]
       │
       ▼ (지원 확장자 필터링)
[경로 배열 생성]
       │
       ▼ (Rayon par_iter 병렬 분기)
┌──────────────┬──────────────┬──────────────┐
│  Worker 1    │  Worker 2    │  Worker N    │
│  - decode    │  - decode    │  - decode    │
│  - crop      │  - crop      │  - crop      │
│  - encode    │  - encode    │  - encode    │
└──────────────┴──────────────┴──────────────┘
       │
       ▼ (Atomic 카운트 집계)
[작업 완료 및 통계 출력]

```
