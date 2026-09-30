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
Sensitive values for the chart Secret, by logical key, only those set
*/}}
{{- define "app.secretValues" -}}
{{- $v := .Values -}}
{{- $db := $v.apiDatabase | default dict -}}
{{- $out := dict -}}
{{- with $v.databaseUrl }}{{ $_ := set $out "databaseUrl" (toString .) }}{{ end -}}
{{- with $v.pgPassword }}{{ $_ := set $out "databasePassword" (toString .) }}{{ end -}}
{{- with $db.url }}{{ $_ := set $out "apiDatabaseUrl" (toString .) }}{{ end -}}
{{- with $db.password }}{{ $_ := set $out "apiDatabasePassword" (toString .) }}{{ end -}}
{{- with $v.apiKey }}{{ $_ := set $out "apiKey" (toString .) }}{{ end -}}
{{- with $v.inseeCredentials }}{{ $_ := set $out "inseeCredentials" (toString .) }}{{ end -}}
{{- with $v.sentryDsn }}{{ $_ := set $out "sentryDsn" (toString .) }}{{ end -}}
{{- toYaml $out }}
{{- end }}

{{/*
Data of a chart Secret holding the given logical keys
  (dict "root" . "keys" (list "databasePassword" ...))
*/}}
{{- define "app.secretData" -}}
{{- $values := include "app.secretValues" .root | fromYaml -}}
{{- range $key := .keys }}
{{- with get $values $key }}
{{ include "app.secretKey" (dict "root" $.root "key" $key) }}: {{ . | b64enc | quote }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Name of a logical key in the Secret, defaulting to the variable name
*/}}
{{- define "app.secretKey" -}}
{{- $defaults := dict "databaseUrl" "DATABASE_URL" "apiDatabaseUrl" "API_DATABASE_URL" "databasePassword" "DATABASE_PASSWORD" "apiDatabasePassword" "API_DATABASE_PASSWORD" "apiKey" "API_KEY" "inseeCredentials" "INSEE_CREDENTIALS" "sentryDsn" "SENTRY_DSN" -}}
{{- $keys := .root.Values.secrets.keys | default dict -}}
{{- get $keys .key | default (get $defaults .key) -}}
{{- end }}

{{/*
An environment variable read from the Secret, omitted when not configured
  (dict "root" . "env" "API_KEY" "key" "apiKey" ["secret" "<name>"])
With an existing Secret, every key is optional.
*/}}
{{- define "app.secretEnv" -}}
{{- $existing := .root.Values.secrets.existingSecret -}}
{{- if or $existing (hasKey (include "app.secretValues" .root | fromYaml) .key) }}
- name: {{ .env | quote }}
  valueFrom:
    secretKeyRef:
      name: {{ $existing | default .secret | default (include "app.fullname" .root) | quote }}
      key: {{ include "app.secretKey" . | quote }}
      {{- if $existing }}
      optional: true
      {{- end }}
{{- end }}
{{- end }}

{{/*
A plain environment variable, omitted when empty so that envFrom can provide it
*/}}
{{- define "app.optionalEnv" -}}
{{- if not (kindIs "invalid" .value) }}{{ if ne (toString .value) "" }}
- name: {{ .name | quote }}
  value: {{ toString .value | quote }}
{{- end }}{{ end }}
{{- end }}

{{/*
Database connection of the updater and the migration job: always the primary
  (dict "root" . ["secret" "<name>"])
*/}}
{{- define "app.databaseEnv" -}}
{{- $v := .root.Values -}}
{{- /* The app prefers DATABASE_URL over the separate fields. */ -}}
{{- include "app.secretEnv" (dict "root" .root "env" "DATABASE_URL" "key" "databaseUrl" "secret" .secret) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_HOST" "value" $v.pgHost) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_PORT" "value" $v.pgPort) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_NAME" "value" $v.pgDatabase) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_USER" "value" $v.pgUsername) }}
{{- include "app.secretEnv" (dict "root" .root "env" "DATABASE_PASSWORD" "key" "databasePassword" "secret" .secret) }}
{{- end }}

{{/*
Database connection of the API only: apiDatabase, field by field, over the
primary. The migration Job and the updater keep writing to the primary.
*/}}
{{- define "app.apiDatabaseEnv" -}}
{{- $v := .Values -}}
{{- $db := $v.apiDatabase | default dict -}}
{{- $existing := not (empty $v.secrets.existingSecret) -}}
{{- $keys := $v.secrets.keys | default dict -}}
{{- if and (or $db.host $db.url) (not $v.migrations.enabled) -}}
{{- fail "apiDatabase requires migrations.enabled: the API cannot migrate a read replica on startup" -}}
{{- end -}}
{{- /* A replica URL when one is given (apiDatabase.url, or its key in the existing Secret). */ -}}
{{- $replicaUrl := ternary (get $keys "apiDatabaseUrl") $db.url $existing -}}
{{- if $replicaUrl }}
{{- include "app.secretEnv" (dict "root" . "env" "DATABASE_URL" "key" "apiDatabaseUrl") }}
{{- else if $db.host }}
{{- if and $v.databaseUrl (not $v.pgHost) -}}
{{- fail "apiDatabase.host cannot fall back on databaseUrl field by field: give the replica as apiDatabase.url" -}}
{{- end -}}
{{- /* Replica fields: no DATABASE_URL, which would win over them. */ -}}
{{- else }}
{{- include "app.secretEnv" (dict "root" . "env" "DATABASE_URL" "key" "databaseUrl") }}
{{- end }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_HOST" "value" ($db.host | default $v.pgHost)) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_PORT" "value" ($db.port | default $v.pgPort)) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_NAME" "value" ($db.database | default $v.pgDatabase)) }}
{{- include "app.optionalEnv" (dict "name" "DATABASE_USER" "value" ($db.username | default $v.pgUsername)) }}
{{- /* The replica password when one is given, the primary's otherwise. */ -}}
{{- $replicaPassword := ternary (get $keys "apiDatabasePassword") $db.password $existing -}}
{{- include "app.secretEnv" (dict "root" . "env" "DATABASE_PASSWORD" "key" (ternary "apiDatabasePassword" "databasePassword" (not (empty $replicaPassword)))) }}
{{- end }}

{{/*
Environment shared by every container: logs, Sentry, then the user's extraEnv
  (dict "root" . ["secret" "<name>"])
*/}}
{{- define "app.commonEnv" -}}
{{- $v := .root.Values -}}
- name: "RUST_LOG"
  value: {{ $v.rustLog | quote }}
- name: "SIRENE_ENV"
  value: {{ $v.sireneEnv | quote }}
{{- include "app.secretEnv" (dict "root" .root "env" "SENTRY_DSN" "key" "sentryDsn" "secret" .secret) }}
{{- with $v.extraEnv }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{- define "app.envFrom" -}}
{{- with .Values.envFrom }}
envFrom:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}

{{/*
Labels of the job pods: never the selector labels, or the Service would pick
them up (and the geocoding pod affinity would target them)
*/}}
{{- define "app.jobPodLabels" -}}
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .component }}
{{- with .root.Values.podLabels }}
{{ toYaml . }}
{{- end }}
{{- end }}

{{/*
Pod spec fields shared by every workload
  (dict "root" . "jobs" <bool> "geocoding" <bool>)
- jobs: scheduling from `jobs.*`, falling back to the top-level values.
- geocoding: joins the node of the API pods when the ReadWriteOnce geocoding
  volume must be shared (geocoding.persistence.sameNode).
*/}}
{{- define "app.podCommon" -}}
{{- $v := .root.Values -}}
{{- $jobs := $v.jobs | default dict -}}
{{- $nodeSelector := $v.nodeSelector -}}
{{- $tolerations := $v.tolerations -}}
{{- $affinity := deepCopy ($v.affinity | default dict) -}}
{{- if .jobs -}}
{{- $nodeSelector = $jobs.nodeSelector | default $nodeSelector -}}
{{- $tolerations = $jobs.tolerations | default $tolerations -}}
{{- $affinity = deepCopy ($jobs.affinity | default $affinity) -}}
{{- end -}}
{{- if and .geocoding $v.geocoding.enabled $v.geocoding.persistence.sameNode -}}
{{- $term := dict "labelSelector" (dict "matchLabels" (include "app.selectorLabels" .root | fromYaml)) "topologyKey" "kubernetes.io/hostname" -}}
{{- $podAffinity := get $affinity "podAffinity" | default dict -}}
{{- $required := get $podAffinity "requiredDuringSchedulingIgnoredDuringExecution" | default list -}}
{{- $_ := set $podAffinity "requiredDuringSchedulingIgnoredDuringExecution" (append $required $term) -}}
{{- $_ := set $affinity "podAffinity" $podAffinity -}}
{{- end -}}
automountServiceAccountToken: {{ $v.serviceAccount.automount }}
{{- with $v.imagePullSecrets }}
imagePullSecrets:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with $v.priorityClassName }}
priorityClassName: {{ . }}
{{- end }}
{{- with $v.podSecurityContext }}
securityContext:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with $nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with $tolerations }}
tolerations:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- with $affinity }}
affinity:
  {{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}

{{/*
When the migration hook owns the schema, the other workloads only check it
*/}}
{{- define "app.skipMigrationsEnv" -}}
- name: "SKIP_MIGRATIONS"
  value: "{{ .Values.migrations.enabled }}"
{{- end }}

{{/*
Geocoding: claim name, environment and volume of the index
*/}}
{{- define "app.geocodingClaim" -}}
{{- .Values.geocoding.persistence.existingClaim | default (printf "%s-geocoding" (include "app.fullname" .)) }}
{{- end }}

{{- define "app.geocodingEnv" -}}
{{- if .Values.geocoding.enabled }}
- name: "GEOCODING_INDEX_PATH"
  value: {{ .Values.geocoding.indexPath | quote }}
- name: "GEOCODING_SOURCE_URL"
  value: {{ .Values.geocoding.sourceUrl | quote }}
- name: "GEOCODING_WITH_UPDATE_ALL"
  value: "{{ .Values.geocoding.withUpdateAll }}"
- name: "GEOCODING_RELOAD_INTERVAL_SECONDS"
  value: "{{ .Values.geocoding.reloadIntervalSeconds }}"
{{- end }}
{{- end }}

{{- define "app.geocodingVolumeMount" -}}
{{- if .Values.geocoding.enabled }}
- name: geocoding
  mountPath: {{ dir .Values.geocoding.indexPath }}
{{- end }}
{{- end }}

{{- define "app.geocodingVolume" -}}
{{- if .Values.geocoding.enabled }}
- name: geocoding
  persistentVolumeClaim:
    claimName: {{ include "app.geocodingClaim" . }}
{{- end }}
{{- end }}

{{/*
Values replaced in chart 1.3.0: fail loudly rather than silently drop routing
*/}}
{{- define "app.checkRemovedValues" -}}
{{- if hasKey .Values "gateway" -}}
{{- fail "gateway.* was split into listenerSet.* (enabled, annotations, parentRef, hosts, tls) and httpRoute.* (enabled, parentRefs, hostnames, matches, filters, extraRules)" -}}
{{- end -}}
{{- if hasKey .Values "extAuth" -}}
{{- fail "extAuth.* moved to auth.*: provider kgateway, auth.kgateway.extensionRef, and the protected paths in auth.matches (/v3 by default)" -}}
{{- end -}}
{{- end }}

{{- define "app.authPolicyName" -}}
{{- printf "%s-auth" (include "app.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}
