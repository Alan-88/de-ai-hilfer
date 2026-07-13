import type { AiModelOverride, AiSettingsResponse } from "$lib/types";

export type ActionModelOption = AiModelOverride & {
  key: string;
  label: string;
};

export function buildActionModelOptions(settings: AiSettingsResponse | null): ActionModelOption[] {
  if (!settings) return [];
  return settings.profiles.flatMap((profile) =>
    profile.model_ids
      .filter((model) => !model.toLowerCase().includes("embedding"))
      .map((model) => ({
        key: `${profile.name}:${model}`,
        label: `${profile.name} / ${model}`,
        provider_name: profile.name,
        model_id: model,
      }))
  );
}

export function defaultActionModelKey(settings: AiSettingsResponse | null): string {
  if (!settings) return "";
  const analyze = settings.task_settings.find((setting) => setting.task_key === "analyze");
  if (analyze?.provider_name && analyze.model_id && !analyze.model_id.toLowerCase().includes("embedding")) {
    return `${analyze.provider_name}:${analyze.model_id}`;
  }
  return buildActionModelOptions(settings)[0]?.key ?? "";
}

export function actionModelOverrideForKey(options: ActionModelOption[], key: string): AiModelOverride | null {
  const option = options.find((item) => item.key === key);
  return option ? { provider_name: option.provider_name, model_id: option.model_id } : null;
}
