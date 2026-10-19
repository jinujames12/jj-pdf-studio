// Which page (by item id) the large preview dialog is showing, if any.
let openId = $state<number | null>(null);

export const preview = {
  get id() {
    return openId;
  },
  open(id: number) {
    openId = id;
  },
  close() {
    openId = null;
  }
};
