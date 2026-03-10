<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# settingscli YAML 명령어 API Server 직접 라우팅


**문서 번호**: PICCOLO-SETTINGSCLI-YAML-ROUTING-2026-001
**버전**: 1.0
**날짜**: 2026-03-06
**작성자**: PICCOLO 팀
**분류**: LLD (Low-Level Design)

---

## 0. 문서의 목적

- **목표**: settingscli의 YAML 배포 및 삭제 명령어를 API Server로 직접 전송
- **왜 필요한가**: ARM 보드 환경에서 SettingsService 중계 시 YAML 전달 오류 발생
- 이 문서는 settingscli 컴포넌트에 YAML 명령어를 API Server로 직접 라우팅하는 기능을 추가하기 위해 작성되었습니다.
- (상세 설명) 기존에는 모든 명령어가 SettingsService(8080)로 전송되었으나, YAML 관련 명령어만 API Server(47099)로 직접 전송하도록 변경합니다.
- 이 기능은 이 문서에 포함된 조건 및 규칙들을 따라야 합니다.

**As-Is:**
```
settingscli → SettingsService (8080) → API Server (47099)
(모든 명령어) (YAML 중계)
```

**To-Be:**
```
settingscli → SettingsService (8080) (metrics, board, node 등)
└→ API Server (47099) (yaml 명령어만)
```

---

## 1. settingscli의 기능

- **기능 1: 시스템 관리 명령어 처리**
- metrics, board, node, soc, container 등의 명령어를 SettingsService로 전송
- 시스템 상태 조회 및 관리 작업 수행

- **기능 2: YAML 아티팩트 배포/삭제**
- `yaml apply`: YAML 파일을 읽어 시스템에 배포
- `yaml withdraw`: YAML 파일 기반으로 리소스 삭제
- Scenario, Package, Model 등 다중 리소스 관리

- **기능 3: 헬스 체크**
- SettingsService와 API Server의 상태 확인
- 서비스 가용성 검증

---

## 2. settingscli의 구현 구조

```
settingscli/
├── src/
│ ├── main.rs (변경 대상)
│ │ └─> CLI 파싱 및 클라이언트 생성
│ │ └─> 명령어 라우팅 로직
│ ├── lib.rs
│ ├── client.rs (기존 SettingsClient 사용)
│ ├── error.rs
│ └── commands/
│ ├── yaml.rs (변경 대상)
│ ├── metrics.rs
│ ├── board.rs
│ ├── node.rs
│ ├── soc.rs
│ └── container.rs
```

**파일별 역할:**

- **main.rs**: settingscli 실행의 진입점입니다. CLI 인자 파싱, 클라이언트 생성, 명령어 라우팅을 담당합니다.

- **client.rs**: HTTP 클라이언트 구현체입니다. SettingsService와 API Server 모두 동일한 `SettingsClient` 구조체를 사용하되, URL만 다르게 설정합니다.

- **yaml.rs**: YAML 명령어(apply, withdraw) 핸들러입니다. API Server의 엔드포인트로 요청을 전송합니다.

- **commands/*.rs**: 각 명령어별 핸들러들입니다. SettingsService를 사용하는 기존 명령어들입니다.

---

## 3. YAML 명령어를 위해 settingscli에 구현되어야 하는 것

```
+-------------------+ +---------------------+ +-------------------+
| settingscli | HTTP | API Server | 처리 | ETCD |
| (yaml 명령) | ------> | (YAML 처리) | ------> | (상태 저장) |
+-------------------+ +---------------------+ +-------------------+
(Port 47099)

+-------------------+ +---------------------+
| settingscli | HTTP | SettingsService |
| (기타 명령) | ------> | (시스템 관리) |
+-------------------+ +---------------------+
(Port 8080)
```

### 3.1 인터페이스

- **입력**: 사용자로부터 CLI 명령어 수신
- `settingscli yaml apply <file>`: YAML 파일 배포
- `settingscli yaml withdraw <file>`: YAML 파일 기반 삭제

- **처리**:
- YAML 파일 읽기 및 검증
- 명령어 타입에 따라 적절한 클라이언트(SettingsService 또는 API Server) 선택
- HTTP 요청 전송

- **출력**:
- API Server(47099)에 YAML 내용 전송 (yaml 명령어)
- SettingsService(8080)에 요청 전송 (기타 명령어)

### 3.2 변경 대상 상세

#### 3.2.1 main.rs 변경사항

**핵심 변경:**
1. CLI 구조: 단일 URL → Base URL + 2 ports (settings_port: 8080, api_port: 47099)
2. 클라이언트: 단일 client → settings_client + api_client
3. 라우팅: YAML 명령어는 api_client로, 나머지는 settings_client로

```rust
// CLI 구조
struct Cli {
#[arg(short, long, env = "PICCOLO_URL", default_value = "http://localhost")]
url: String,
#[arg(long, env = "SETTINGS_PORT", default_value = "8080")]
settings_port: u16,
#[arg(long, env = "API_PORT", default_value = "47099")]
api_port: u16,
}

// 이중 클라이언트 생성
let settings_client = SettingsClient::new(&settings_url, cli.timeout)?;
let api_client = SettingsClient::new(&api_url, cli.timeout)?;

// 명령어 라우팅
let result = match cli.command {
Commands::Yaml { action } => yaml::handle(&api_client, action).await, // API Server
Commands::Metrics { action } => metrics::handle(&settings_client, action).await, // SettingsService
Commands::Health => health_check(&settings_client, &api_client).await, // 양쪽 모두
...
};
```

#### 3.2.2 yaml.rs 변경사항

**핵심 변경: 엔드포인트만 변경**

```rust
// Before
client.post_yaml("/api/v1/yaml", &yaml_content).await?; // SettingsService
client.delete_yaml("/api/v1/yaml", &yaml_content).await?; // SettingsService

// After
client.post_yaml("/api/artifact", &yaml_content).await?; // API Server 직접
client.delete_yaml("/api/artifact", &yaml_content).await?; // API Server 직접
```

**참고:** 파일 읽기, 검증 로직 등 나머지 코드는 모두 동일

---

## 4. 지켜야 할 규칙

### 4.1 URL 및 포트 규칙

- **SettingsService와 API Server의 IP는 동일**
- 같은 호스트에서 실행되지만 포트만 다름
- 예: `http://10.231.178.2:8080` (SettingsService), `http://10.231.178.2:47099` (API Server)

- **포트 기본값**
- SettingsService: `8080`
- API Server: `47099`

- **환경 변수 지원**
```bash
export PICCOLO_URL=http://10.231.178.2
export SETTINGS_PORT=8080
export API_PORT=47099
```

### 4.2 엔드포인트 변경 규칙

| 명령어 | Before (SettingsService) | After (API Server) | Method |
|--------|-------------------------|-------------------|---------|
| `yaml apply` | `/api/v1/yaml` | `/api/artifact` | POST |
| `yaml withdraw` | `/api/v1/yaml` | `/api/artifact` | DELETE |

### 4.3 클라이언트 사용 규칙

- **동일한 SettingsClient 구조체 재사용**
- client.rs의 기존 구현 그대로 사용
- URL만 다르게 설정하여 두 개의 인스턴스 생성

- **명령어별 클라이언트 선택**
```rust
// YAML 명령어 → api_client
Commands::Yaml { action } => yaml::handle(&api_client, action).await,

// 기타 명령어 → settings_client
Commands::Metrics { action } => metrics::handle(&settings_client, action).await,
```

### 4.4 Health Check 규칙

- **양쪽 서비스 모두 확인 필요**
- SettingsService: `/api/v1/health` 확인
- API Server: `/api/health` 확인
- 둘 중 하나라도 실패 시 에러 반환

### 4.5 에러 처리 규칙

- 실패 시 명확한 에러 메시지 출력 (기존 방식 유지)
- 파일 읽기 실패, HTTP 요청 실패 등 모두 사용자 친화적 메시지 표시

---