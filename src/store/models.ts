import type { Models } from "@rematch/core";
import type { feedModel } from "./models/feed";
import type { playerModel } from "./models/player";
import type { settingsModel } from "./models/settings";

export interface RootModel extends Models<RootModel> {
  feed: typeof feedModel;
  player: typeof playerModel;
  settings: typeof settingsModel;
}
