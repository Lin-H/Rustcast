import { init } from "@rematch/core";
import type { RematchDispatch, RematchRootState } from "@rematch/core";
import { useEffect, useState } from "preact/hooks";
import type { RootModel } from "./models";
export type { RootModel };
import { feedModel } from "./models/feed";
import { playerModel } from "./models/player";
import { settingsModel } from "./models/settings";

export const store = init<RootModel>({
  models: {
    feed: feedModel,
    player: playerModel,
    settings: settingsModel,
  },
});

export type RootState = RematchRootState<RootModel>;
export type AppDispatch = RematchDispatch<RootModel>;
export const dispatch = store.dispatch as AppDispatch;

export function useAppSelector<T>(selector: (state: RootState) => T): T {
  const [snapshot, setSnapshot] = useState(() => selector(store.getState()));

  useEffect(() => {
    const update = () => {
      const next = selector(store.getState());
      setSnapshot((current) => (Object.is(current, next) ? current : next));
    };

    update();
    return store.subscribe(update);
  }, [selector]);

  return snapshot;
}
