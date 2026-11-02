import { PLUGINS_UNAVAILABLE_MESSAGE } from "$lib/personal-build-policy";

export async function pluginInvoke<T>(
  _pluginId: string,
  _command: string,
  _args: Record<string, unknown> = {},
): Promise<T> {
  throw new Error(PLUGINS_UNAVAILABLE_MESSAGE);
}
