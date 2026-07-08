<script lang="ts">
  import { onMount } from "svelte";
  import { getAiSettings, updateAiSettings } from "$lib/queryApi";
  import type {
    AiProviderProfileInput,
    AiProviderProfileView,
    AiSettingsResponse,
    AiTaskModelSettingInput,
    AiTaskModelSettingView,
  } from "$lib/types";

  type TaskKey = AiTaskModelSettingView["task_key"];
  type EditableProfile = AiProviderProfileInput & {
    client_id: string;
    api_key_set?: boolean;
    api_key_preview?: string | null;
    model_draft: string;
  };

  const INHERIT_ANALYZE_VALUE = "__inherit_analyze__";

  const taskMeta: Array<{ key: TaskKey; label: string; icon: string; tone: string }> = [
    { key: "analyze", label: "查词", icon: "ph ph-sparkle", tone: "tone-analyze" },
    { key: "phrase", label: "短语", icon: "ph ph-puzzle-piece", tone: "tone-phrase" },
    { key: "structure", label: "结构化输出", icon: "ph ph-brackets-curly", tone: "tone-structure" },
    { key: "embedding", label: "Embedding", icon: "ph ph-dots-three-circle", tone: "tone-embedding" },
    { key: "intelligent_search", label: "联想搜索", icon: "ph ph-compass", tone: "tone-search" },
  ];

  let profiles = $state<EditableProfile[]>([]);
  let taskSettings = $state<Record<TaskKey, AiTaskModelSettingInput>>(emptyTaskSettings());
  let isLoading = $state(true);
  let isSaving = $state(false);
  let error = $state("");
  let success = $state("");

  const hasProfiles = $derived(profiles.length > 0);

  onMount(() => {
    loadSettings();
  });

  async function loadSettings() {
    isLoading = true;
    error = "";
    success = "";

    try {
      applySettings(await getAiSettings());
    } catch (loadError) {
      error = loadError instanceof Error ? loadError.message : "模型配置加载失败";
    } finally {
      isLoading = false;
    }
  }

  function applySettings(settings: AiSettingsResponse) {
    profiles = settings.profiles.map(toEditableProfile);
    taskSettings = normalizeTaskSettings(settings.task_settings);
  }

  function toEditableProfile(profile: AiProviderProfileView): EditableProfile {
    return {
      id: profile.id,
      client_id: profile.id ? `profile-${profile.id}` : crypto.randomUUID(),
      name: profile.name,
      base_url: profile.base_url,
      api_key: null,
      model_ids: [...profile.model_ids],
      model_draft: "",
      is_default: profile.is_default,
      api_key_set: profile.api_key_set,
      api_key_preview: profile.api_key_preview,
    };
  }

  function emptyTaskSettings(): Record<TaskKey, AiTaskModelSettingInput> {
    return {
      analyze: { task_key: "analyze", provider_name: null, model_id: null, inherit_task_key: null },
      phrase: { task_key: "phrase", provider_name: null, model_id: null, inherit_task_key: "analyze" },
      structure: { task_key: "structure", provider_name: null, model_id: null, inherit_task_key: null },
      embedding: { task_key: "embedding", provider_name: null, model_id: null, inherit_task_key: null },
      intelligent_search: { task_key: "intelligent_search", provider_name: null, model_id: null, inherit_task_key: null },
    };
  }

  function normalizeTaskSettings(settings: AiTaskModelSettingView[]) {
    const next = emptyTaskSettings();
    for (const setting of settings) {
      next[setting.task_key] = {
        task_key: setting.task_key,
        provider_name: setting.provider_name ?? null,
        model_id: setting.model_id ?? null,
        inherit_task_key: setting.inherit_task_key ?? null,
      };
    }
    return next;
  }

  function addProfile() {
    const profile: EditableProfile = {
      client_id: crypto.randomUUID(),
      name: uniqueProfileName("新供应商"),
      base_url: "https://api.openai.com/v1",
      api_key: "",
      model_ids: [],
      model_draft: "",
      is_default: profiles.length === 0,
      api_key_set: false,
      api_key_preview: null,
    };
    profiles = [...profiles, profile];
  }

  function removeProfile(clientId: string) {
    const removed = profiles.find((profile) => profile.client_id === clientId);
    profiles = profiles.filter((profile) => profile.client_id !== clientId);
    if (removed) {
      for (const task of taskMeta) {
        const setting = taskSettings[task.key];
        if (setting.provider_name === removed.name) {
          updateTask(task.key, { provider_name: profiles[0]?.name ?? null, model_id: profiles[0]?.model_ids?.[0] ?? null });
        }
      }
    }
  }

  function updateProfile(clientId: string, patch: Partial<EditableProfile>) {
    profiles = profiles.map((profile) =>
      profile.client_id === clientId ? { ...profile, ...patch } : profile
    );
  }

  function renameProfile(profile: EditableProfile, name: string) {
    updateProfile(profile.client_id, { name });
    for (const task of taskMeta) {
      const setting = taskSettings[task.key];
      if (setting.provider_name === profile.name) {
        updateTask(task.key, { provider_name: name });
      }
    }
  }

  function addModel(profile: EditableProfile) {
    const model = profile.model_draft.trim();
    if (!model || profile.model_ids?.includes(model)) return;
    updateProfile(profile.client_id, {
      model_ids: [...(profile.model_ids ?? []), model],
      model_draft: "",
    });
  }

  function removeModel(profile: EditableProfile, model: string) {
    updateProfile(profile.client_id, {
      model_ids: (profile.model_ids ?? []).filter((item) => item !== model),
    });
    for (const task of taskMeta) {
      const setting = taskSettings[task.key];
      if (setting.provider_name === profile.name && setting.model_id === model) {
        updateTask(task.key, { model_id: null });
      }
    }
  }

  function updateTask(taskKey: TaskKey, patch: Partial<AiTaskModelSettingInput>) {
    taskSettings = {
      ...taskSettings,
      [taskKey]: {
        ...taskSettings[taskKey],
        ...patch,
      },
    };
  }

  function setTaskProvider(taskKey: TaskKey, providerName: string) {
    if (taskKey === "phrase" && providerName === INHERIT_ANALYZE_VALUE) {
      updateTask("phrase", { inherit_task_key: "analyze", provider_name: null, model_id: null });
      return;
    }
    const profile = profiles.find((item) => item.name === providerName);
    updateTask(taskKey, {
      provider_name: providerName || null,
      model_id: profile?.model_ids?.[0] ?? null,
      inherit_task_key: null,
    });
  }

  function modelOptions(providerName: string | null | undefined) {
    return profiles.find((profile) => profile.name === providerName)?.model_ids ?? [];
  }

  function uniqueProfileName(base: string) {
    let index = 1;
    let candidate = base;
    const names = new Set(profiles.map((profile) => profile.name));
    while (names.has(candidate)) {
      index += 1;
      candidate = `${base} ${index}`;
    }
    return candidate;
  }

  function validateBeforeSave() {
    if (profiles.length === 0) return "至少保留一个供应商 Profile。";

    const names = new Set<string>();
    for (const profile of profiles) {
      if (!profile.name.trim()) return "Profile 名称不能为空。";
      if (!profile.base_url.trim()) return `${profile.name} 缺少 Base URL。`;
      if (!profile.id && profile.api_key_set && !profile.api_key?.trim()) {
        return `${profile.name} 来自环境变量，首次保存前需要重新填写 API Key。`;
      }
      if (names.has(profile.name.trim())) return `Profile 名称重复：${profile.name}`;
      names.add(profile.name.trim());
    }

    for (const task of taskMeta) {
      const setting = taskSettings[task.key];
      if (setting.inherit_task_key) continue;
      if (!setting.provider_name) return `${task.label} 缺少供应商。`;
      const profile = profiles.find((item) => item.name === setting.provider_name);
      if (!profile) return `${task.label} 指向了不存在的 Profile。`;
      if (!setting.model_id) return `${task.label} 缺少模型。`;
      if (!(profile.model_ids ?? []).includes(setting.model_id)) {
        return `${task.label} 指向了不在 Profile 中的模型。`;
      }
    }
    return "";
  }

  async function saveSettings() {
    error = "";
    success = "";
    const validationError = validateBeforeSave();
    if (validationError) {
      error = validationError;
      return;
    }

    isSaving = true;
    try {
      const payload = buildPayload();
      const response = await updateAiSettings(payload);
      applySettings(response);
      success = "模型配置已保存。";
    } catch (saveError) {
      error = saveError instanceof Error ? saveError.message : "模型配置保存失败";
    } finally {
      isSaving = false;
    }
  }

  function buildPayload() {
    return {
      profiles: profiles.map((profile) => ({
        id: profile.id ?? null,
        name: profile.name.trim(),
        base_url: profile.base_url.trim(),
        api_key: profile.api_key?.trim() ? profile.api_key.trim() : null,
        model_ids: profile.model_ids ?? [],
        is_default: profiles[0]?.client_id === profile.client_id,
      })),
      task_settings: taskMeta.map(({ key }) => {
        const setting = taskSettings[key];
        if (setting.inherit_task_key) {
          return { task_key: key, inherit_task_key: setting.inherit_task_key };
        }
        return {
          task_key: key,
          provider_name: setting.provider_name ?? null,
          model_id: setting.model_id ?? null,
          inherit_task_key: null,
        };
      }),
    };
  }
</script>

<section class="bento-card ai-settings-card">
  <div class="section-head">
    <div>
      <div class="card-title"><i class="ph-fill ph-sliders-horizontal"></i> 模型配置</div>
      <h2>AI 调用路由</h2>
    </div>
    <div class="actions">
      <button type="button" class="icon-btn" title="重新加载" onclick={loadSettings} disabled={isLoading || isSaving}>
        <i class="ph ph-arrow-clockwise" class:spin={isLoading}></i>
      </button>
      <button type="button" class="btn-primary" onclick={saveSettings} disabled={isLoading || isSaving || !hasProfiles}>
        <i class="ph ph-floppy-disk"></i>
        {isSaving ? "保存中" : "保存配置"}
      </button>
    </div>
  </div>

  {#if error}
    <div class="message-surface error">{error}</div>
  {/if}

  {#if success}
    <div class="message-surface success">{success}</div>
  {/if}

  {#if isLoading}
    <div class="loading-stack">
      <div class="skeleton-block"></div>
      <div class="skeleton-block short"></div>
      <div class="skeleton-block"></div>
    </div>
  {:else}
    <div class="settings-layout">
      <div class="profiles-column">
        <div class="subhead">
          <span>供应商 Profile</span>
          <button type="button" class="icon-btn add" title="新增 Profile" onclick={addProfile}>
            <i class="ph ph-plus"></i>
          </button>
        </div>

        <div class="profile-list">
          {#each profiles as profile (profile.client_id)}
            <article class="profile-card">
              <div class="profile-top">
                <label class="field name-field">
                  <span>名称</span>
                  <input
                    value={profile.name}
                    oninput={(event) => renameProfile(profile, event.currentTarget.value)}
                  />
                </label>
                <button
                  type="button"
                  class="icon-btn danger"
                  title="删除 Profile"
                  onclick={() => removeProfile(profile.client_id)}
                  disabled={profiles.length <= 1}
                >
                  <i class="ph ph-trash"></i>
                </button>
              </div>

              <label class="field">
                <span>Base URL</span>
                <input
                  value={profile.base_url}
                  oninput={(event) => updateProfile(profile.client_id, { base_url: event.currentTarget.value })}
                />
              </label>

              <label class="field">
                <span>API Key</span>
                <input
                  type="password"
                  value={profile.api_key ?? ""}
                  oninput={(event) => updateProfile(profile.client_id, { api_key: event.currentTarget.value })}
                />
              </label>

              <div class="model-editor">
                <span class="field-label">模型 ID</span>
                <div class="model-chips">
                  {#each profile.model_ids ?? [] as model}
                    <button type="button" class="model-chip" title="移除模型" onclick={() => removeModel(profile, model)}>
                      <span>{model}</span>
                      <i class="ph ph-x"></i>
                    </button>
                  {/each}
                  {#if (profile.model_ids ?? []).length === 0}
                    <span class="empty-chip">暂无模型</span>
                  {/if}
                </div>
                <div class="inline-add">
                  <input
                    placeholder="输入模型 id"
                    value={profile.model_draft}
                    oninput={(event) => updateProfile(profile.client_id, { model_draft: event.currentTarget.value })}
                    onkeydown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        addModel(profile);
                      }
                    }}
                  />
                  <button type="button" class="icon-btn add" title="添加模型" onclick={() => addModel(profile)}>
                    <i class="ph ph-plus"></i>
                  </button>
                </div>
              </div>
            </article>
          {/each}
        </div>
      </div>

      <div class="tasks-column">
        <div class="subhead"><span>任务默认模型</span></div>
        <div class="task-grid">
          {#each taskMeta as task}
            {@const setting = taskSettings[task.key]}
            {@const models = modelOptions(setting.provider_name)}
            {@const providerValue = task.key === "phrase" && setting.inherit_task_key === "analyze" ? INHERIT_ANALYZE_VALUE : setting.provider_name ?? ""}
            <article class={`task-row ${task.tone}`}>
              <div class="task-label">
                <i class={task.icon}></i>
                <span>{task.label}</span>
              </div>

              <select value={providerValue} onchange={(event) => setTaskProvider(task.key, event.currentTarget.value)}>
                <option value="" disabled>选择 Profile</option>
                {#if task.key === "phrase"}
                  <option value={INHERIT_ANALYZE_VALUE}>沿用查词</option>
                {/if}
                {#each profiles as profile}
                  <option value={profile.name}>{profile.name}</option>
                {/each}
              </select>

              {#if setting.inherit_task_key}
                <select value={INHERIT_ANALYZE_VALUE} disabled>
                  <option value={INHERIT_ANALYZE_VALUE}>查词模型</option>
                </select>
              {:else}
                <select
                  value={setting.model_id ?? ""}
                  onchange={(event) => updateTask(task.key, { model_id: event.currentTarget.value || null })}
                  disabled={models.length === 0}
                >
                  <option value="" disabled>选择模型</option>
                  {#each models as model}
                    <option value={model}>{model}</option>
                  {/each}
                </select>
              {/if}
            </article>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .ai-settings-card {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .section-head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .section-head h2 {
    font-size: 1.45rem;
    color: var(--text-main);
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .settings-layout {
    display: grid;
    grid-template-columns: minmax(280px, 0.95fr) minmax(360px, 1.15fr);
    gap: 1.25rem;
    align-items: start;
  }

  .profiles-column, .tasks-column {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    min-width: 0;
  }

  .subhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .profile-list {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }

  .profile-card {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    padding: 1rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--card-bg) 88%, var(--bg-color) 12%);
  }

  .profile-top {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.7rem;
    align-items: end;
  }

  .field, .model-editor {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }

  .field span, .field-label {
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 800;
  }

  input, select {
    width: 100%;
    min-height: 2.5rem;
    border: 1px solid var(--border-color);
    border-radius: 10px;
    background: var(--card-bg);
    color: var(--text-main);
    padding: 0 0.75rem;
    font-size: 0.92rem;
  }

  input:focus, select:focus {
    border-color: var(--accent-main);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-main) 16%, transparent);
  }

  .model-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }

  .model-chip, .empty-chip {
    max-width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.55rem;
    border-radius: 8px;
    background: var(--bg-color);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .model-chip span {
    overflow-wrap: anywhere;
  }

  .empty-chip {
    color: var(--text-muted);
    font-weight: 600;
  }

  .inline-add {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.5rem;
  }

  .icon-btn {
    width: 2.5rem;
    height: 2.5rem;
    display: inline-grid;
    place-items: center;
    border-radius: 10px;
    background: var(--btn-secondary);
    color: var(--btn-secondary-text);
    border: 1px solid var(--border-color);
    transition: all var(--transition-fast);
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--accent-main);
    transform: translateY(-1px);
  }

  .icon-btn.add {
    color: var(--accent-main);
  }

  .icon-btn.danger:hover:not(:disabled) {
    color: var(--danger-text);
  }

  .icon-btn:disabled, button:disabled, select:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .task-grid {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .task-row {
    display: grid;
    grid-template-columns: minmax(130px, 0.75fr) minmax(130px, 0.7fr) minmax(180px, 1.2fr);
    gap: 0.65rem;
    align-items: center;
    padding: 0.85rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-color);
    background:
      linear-gradient(90deg, var(--route-color) 0 4px, transparent 4px),
      color-mix(in srgb, var(--card-bg) 92%, var(--bg-color) 8%);
  }

  .task-label {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-width: 0;
    color: var(--text-main);
    font-weight: 800;
  }

  .task-label i {
    color: var(--route-color);
    font-size: 1.1rem;
  }

  .inherited-route {
    min-height: 2.5rem;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.8rem;
    border-radius: 10px;
    color: var(--accent-main);
    background: color-mix(in srgb, var(--accent-main) 9%, transparent);
    font-weight: 800;
    font-size: 0.9rem;
  }

  .tone-analyze { --route-color: var(--accent-main); }
  .tone-phrase { --route-color: #2c9f7b; }
  .tone-structure { --route-color: #7a65d9; }
  .tone-embedding { --route-color: #b6782c; }
  .tone-search { --route-color: #3c87c8; }

  .loading-stack {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .loading-stack .short {
    width: 62%;
  }

  @media (max-width: 1100px) {
    .settings-layout {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 760px) {
    .section-head {
      flex-direction: column;
    }

    .actions {
      width: 100%;
      justify-content: flex-start;
    }

    .task-row {
      grid-template-columns: 1fr;
    }

    .inherited-route {
      grid-column: auto;
    }
  }
</style>
