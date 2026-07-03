# Lifecycle adapter 검증용 시나리오

ActionController → Lifecycle gRPC 어댑터 경로를 검증하기 위한 YAML 세트입니다.

## 📋 파일 구성

**Binary 아티팩트 방식** - 권장 ✅

### 디렉토리 분류

- `basic/` : Binary lifecycle 기본 동작 검증
  - `binary-demo-launch.yaml`
  - `binary-demo-terminate.yaml`
  - `binary-crash-launch.yaml`
- `context/` : 텍스트 파일 기반 context 유지/재개 검증
  - `counter-file-resume-launch.yaml`

**파일 목록**:
- `basic/binary-demo-launch.yaml` - 정상 동작 바이너리 시작
- `basic/binary-demo-terminate.yaml` - 정상 종료 테스트
- `basic/binary-crash-launch.yaml` - 재시작 정책 테스트
- `context/counter-file-resume-launch.yaml` - crash 후 context 재개 테스트

**Binary 타입 사용 장점**:
- lifecycle gRPC와 1:1 매핑
- 설정이 간단하고 직관적
- container/image 설정 불필요
- 단일 파일에 전체 정보 포함

## 📁 파일 상세

### binary-demo-launch.yaml
```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: binary-demo-launch
spec:
  action: launch
  target: binary-demo
---
kind: Binary
metadata:
  name: binary-demo-main
spec:
  path: /bin/sleep
  args: ["300"]
  restartPolicy: Never
  maxRetries: 0
  restartDelaySecs: 0
  node: lge-NUC15CRSU9
```
- **용도**: 정상 동작 바이너리 시작 테스트
- **검증**: launch 성공, PID 할당, 상태 확인
- **동작**: 5분간 sleep 실행, 재시작 안함

### binary-demo-terminate.yaml
```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: binary-demo-terminate
spec:
  action: terminate
  target: binary-demo
---
kind: Binary
metadata:
  name: binary-demo-main
spec:
  path: /bin/sleep
  args: ["300"]
  restartPolicy: Never
```
- **용도**: graceful stop 테스트
- **검증**: 정상 종료, 프로세스 정리
- **동작**: binary-demo 타겟의 모든 프로세스 종료

### binary-crash-launch.yaml
```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: binary-crash-launch
spec:
  action: launch
  target: binary-crash
---
kind: Binary
metadata:
  name: binary-crash-main
spec:
  path: /bin/sh
  args: ["-c", "exit 1"]
  restartPolicy: OnFailure
  maxRetries: 3
  restartDelaySecs: 1
  node: lge-NUC15CRSU9
```
- **용도**: 재시작 정책 검증
- **검증**: 자동 재시작, restart_count 증가, 최대 재시도 후 중단
- **동작**: 즉시 실패(exit 1) → 1초 대기 → 재시작 (최대 3회)

## 🚀 사용 방법

### 통합 테스트 스크립트 (권장) ✅

**스크립트**: `/home/lge/Desktop/pullpiri/examples/lifecycle-test.sh`

**테스트 범위**:
```
Binary YAML → APIServer → ActionController → Lifecycle gRPC → Linux Process
```

**사전 요구사항**:
- APIServer 실행 중
- ActionController 실행 중
- 환경변수: `ACTIONCONTROLLER_WORKLOAD_RUNTIME=lifecycle`

**실행**:
```bash
# 환경변수 설정
export ACTIONCONTROLLER_WORKLOAD_RUNTIME=lifecycle
export LIFECYCLE_GRPC_ENDPOINT=http://127.0.0.1:50051

# (선택) HOST_IP를 명시적으로 지정 가능 (미설정 시 자동 감지)
# export HOST_IP=10.231.178.2

# Lifecycle gRPC 서버 시작 (터미널 1)
cd /home/lge/Desktop/2track/orchestrator
cargo run -p grpc_lifecycle --bin lifecycle_server --release 2>&1 | tee /tmp/lifecycle.log

# APIServer 시작 (터미널 2)
cd /home/lge/Desktop/pullpiri/src
cargo run -p apiserver

# ActionController 시작 (터미널 3)
cd /home/lge/Desktop/pullpiri/src
cargo run -p actioncontroller

# 통합 테스트 실행 (터미널 4)
/home/lge/Desktop/pullpiri/examples/lifecycle-test.sh

# 재시작 로그 확인 (테스트 후)
grep -i "restart\|retry" /tmp/lifecycle.log
```

**검증 항목**:
- ✅ Binary YAML 파싱
- ✅ APIServer artifact 등록
- ✅ ActionController 워크로드 처리
- ✅ Lifecycle gRPC 호출
- ✅ 실제 프로세스 시작/종료
- ✅ 재시작 정책 동작

### 수동 테스트 (개발/디버깅용)

#### 기본 워크플로우

```bash
# API endpoint 설정
HOST_IP="${HOST_IP:-localhost}"
API_URL="http://${HOST_IP}:47099/api/artifact"

# 1. 정상 동작 바이너리 시작
curl -X POST $API_URL \
  -H "Content-Type: application/yaml" \
  -d @binary-demo-launch.yaml

# 2. 상태 확인
# - lifecycle 서버: curl http://localhost:50051 또는 gRPC 클라이언트
# - actioncontroller 로그: journalctl -u actioncontroller -f

# 3. 종료 테스트
curl -X POST $API_URL \
  -H "Content-Type: application/yaml" \
  -d @binary-demo-terminate.yaml

# 4. 재시작 정책 테스트
curl -X POST $API_URL \
  -H "Content-Type: application/yaml" \
  -d @binary-crash-launch.yaml
# → 프로세스가 exit 1로 실패
# → 1초 대기 후 자동 재시작
# → 최대 3회 재시도
# → restart_count 증가 확인
```

### 직접 gRPC 테스트 (선택사항)

ActionController 없이 lifecycle 서버를 직접 테스트:

```bash
# pullpiri 통합 테스트 사용
/home/lge/Desktop/pullpiri/scripts/test_lifecycle_integration.sh

# 또는 grpc_lifecycle 클라이언트 사용
cd /home/lge/Desktop/2track/orchestrator
cargo run -p grpc_lifecycle --bin lifecycle_client -- \
  start --service test-service --binary /bin/sleep --args "60"
```

## 🎯 검증 포인트

| 테스트 항목 | 파일 | 예상 결과 |
|-----------|------|----------|
| 바이너리 시작 | binary-demo-launch.yaml | ✅ PID 할당, Running 상태 |
| 정상 종료 | binary-demo-terminate.yaml | ✅ 프로세스 종료, 상태 삭제 |
| 재시작 정책 | binary-crash-launch.yaml | ✅ 자동 재시작 3회, restart_count 증가 |
| 상태 조회 | gRPC GetStatus | ✅ 서비스명, PID, 상태, 재시작 횟수 확인 |
| 히스토리 | 로그 파일 | ✅ 시작/종료/재시작 이벤트 기록 |

### 상세 검증 절차

1. **시작 검증**
   ```bash
   curl -X POST $API_URL -d @binary-demo-launch.yaml
   # 확인: actioncontroller 로그에서 "Lifecycle Binary start succeeded" 메시지
   # 확인: lifecycle 서버 로그에서 PID 및 instance_id
   ```

2. **상태 검증**
   ```bash
   # lifecycle 서버에서 GetStatus 호출
   # 확인: service_name=binary-demo-main, pid>0, state=Running
   ```

3. **재시작 검증**
   ```bash
   curl -X POST $API_URL -d @binary-crash-launch.yaml
   # 확인: 1초마다 재시작 시도
   # 확인: restart_count가 1, 2, 3으로 증가
   # 확인: 3회 실패 후 PendingRestart 상태로 유지 또는 제거
   ```

4. **종료 검증**
   ```bash
   curl -X POST $API_URL -d @binary-demo-terminate.yaml
   # 확인: "Lifecycle Binary stop succeeded" 메시지
   # 확인: 프로세스 정리 완료
   ```

## 🔧 환경 변수

```bash
# ActionController 설정
export ACTIONCONTROLLER_WORKLOAD_RUNTIME=lifecycle
export LIFECYCLE_GRPC_ENDPOINT=http://127.0.0.1:50051

# 선택적 설정 (Binary artifact에서 개별 설정 가능)
export LIFECYCLE_RESTART_POLICY=on_failure
export LIFECYCLE_MAX_RETRIES=3
export LIFECYCLE_RESTART_DELAY_SECS=1
```

## 📝 주의사항

### Binary 아티팩트 필수 필드
- `metadata.name`: lifecycle 서비스 이름으로 사용
- `spec.path`: 실행 파일 절대 경로 (필수)
- `spec.args`: 명령행 인자 배열 (선택)
- `spec.node`: 실행할 노드명 (환경에 맞게 수정 필요)

### 재시작 정책 설정
- `restartPolicy`: Never | OnFailure | Always
  - **Never**: 종료되어도 재시작 안함
  - **OnFailure**: exit code != 0 일 때만 재시작
  - **Always**: 항상 재시작 (정상 종료도 재시작)
- `maxRetries`: 최대 재시작 횟수 (0 = 무제한)
- `restartDelaySecs`: 재시작 전 대기 시간 (초)

### 노드명 설정
각 YAML의 `spec.node` 값을 실제 환경에 맞게 수정:
```yaml
spec:
  node: lge-NUC15CRSU9  # ← 실제 노드명으로 변경
```

노드명 확인 방법:
```bash
hostname  # 또는 설정 파일 확인
```

### API Endpoint
기본값: `http://localhost:47099/api/artifact`

변경 시:
```bash
API_URL="http://<APISERVER_IP>:47099/api/artifact"
```
