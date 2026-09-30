{{/*
Expand the name of the chart.
*/}}
{{- define "app.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "app.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "app.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "app.labels" -}}
helm.sh/chart: {{ include "app.chart" . }}
{{ include "app.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "app.selectorLabels" -}}
app.kubernetes.io/name: {{ include "app.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "app.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "app.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Database connection, shared by the API, the updater and the migration job
*/}}
{{- define "app.databaseEnv" -}}
- name: "DATABASE_HOST"
  value: {{ .Values.pgHost }}
- name: "DATABASE_PORT"
  value: "{{ .Values.pgPort }}"
- name: "DATABASE_NAME"
  value: {{ .Values.pgDatabase }}
- name: "DATABASE_USER"
  value: {{ .Values.pgUsername }}
- name: "DATABASE_PASSWORD"
  value: {{ .Values.pgPassword }}
{{- end }}

{{/*
Database connection of the API only: apiDatabase, field by field, over the
primary. The migration Job and the updater keep writing to the primary.
*/}}
{{- define "app.apiDatabaseEnv" -}}
{{- $db := .Values.apiDatabase | default dict -}}
{{- if and $db.host (not .Values.migrations.enabled) -}}
{{- fail "apiDatabase.host requires migrations.enabled: the API cannot migrate a read replica on startup" -}}
{{- end -}}
- name: "DATABASE_HOST"
  value: {{ $db.host | default .Values.pgHost }}
- name: "DATABASE_PORT"
  value: "{{ $db.port | default .Values.pgPort }}"
- name: "DATABASE_NAME"
  value: {{ $db.database | default .Values.pgDatabase }}
- name: "DATABASE_USER"
  value: {{ $db.username | default .Values.pgUsername }}
- name: "DATABASE_PASSWORD"
  value: {{ $db.password | default .Values.pgPassword }}
{{- end }}

{{/*
When the migration hook owns the schema, the other workloads only check it
*/}}
{{- define "app.skipMigrationsEnv" -}}
- name: "SKIP_MIGRATIONS"
  value: "{{ .Values.migrations.enabled }}"
{{- end }}
