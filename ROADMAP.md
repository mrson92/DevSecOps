# 기능 개선 로드맵

참고 저장소: [virtualISP/AI-Powered-Threat-Detection-System](https://github.com/virtualISP/AI-Powered-Threat-Detection-System)
(ELK + Ollama AI 기반 위협 탐지 시스템의 `analyzer.py` 분석 흐름에서 수용 가능한 아이디어 도출)

## 1. 탐지별 추천 조치 (recommendation)

- **현황**: 리포트 `summary`에 통계 수치만 포함되어 있고, 각 탐지 유형에 대한 대응 방안이 없다.
- **개선**: 탐지 유형(규칙/심각도)별 조치 권고(recommendation)를 리포트에 추가한다.
  - 규칙 메타데이터(`rules.references`/설명) 또는 심각도 기반 기본 권고 텍스트 자동 생성
  - 예: Brute Force → "해당 IP의 유입 트래픽 차단 및 계정 잠금 정책 강화"
- **효과**: 리포트가 단순 통계에서 액션 오리엔티드(Action-Oriented) 문서로 발전.

## 2. 근거 기반 상세 (evidence)

- **현황**: `top_rules`/`top_ips`는 카운트만 존재하며, 어떤 로그가 매칭됐는지 근거가 없다.
- **개선**: 리포트 `content`의 최상위 위협 항목에 근거 로그/패턴(evidence)을 포함한다.
  - `rule_executions.context`(매칭 로그 JSON)에서 샘플 추출
  - `top_rules`/`top_ips`에 매칭된 요청 경로·패턴·대표 로그 스니펫 첨부
- **효과**: 보고서 수용·신뢰도 향상, 분석가의 확인 작업 시간 단축.

## 3. LLM 요약/해설 결합

- **현황**: 리포트 생성이 결정적(SQLite 쿼리) 통계로만 구성된다.
- **개선**: 기존 AI 에이전트(persons) 흐름을 활용해 보고서에 해설 문단을 덧붙인다.
  - 통계(`summary`)를 입력으로 LLM이 "증가 추세, 주의 대상 위협, 우선 조치"를 요약
  - AI 해설은 실패해도 리포트 생성을 차단하지 않도록 best-effort 처리(미생성 시 통계만 출력)
- **효과**: 판단이 필요한 내용을 자연어로 전달, 정량 데이터 + 정성 해설의 결합.