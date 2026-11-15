const { invoke } = window.__TAURI__.core;

const leftInput = document.getElementById('left-text');
const rightInput = document.getElementById('right-text');
const status = document.getElementById('status');
const rows = document.getElementById('rows');
const tabList = document.getElementById('tab-list');
const addTabButton = document.getElementById('add-tab');
const wrapToggle = document.getElementById('wrap');

const PROMPT = 'Paste text into both boxes to compare.';
const tabs = [{ name: 'Tab 1', left: '', right: '' }];
let activeTab = 0;

/** Copies the visible texts into the active tab so they survive a tab switch. */
function saveActiveTab() {
  tabs[activeTab].left = leftInput.value;
  tabs[activeTab].right = rightInput.value;
}

/** Renders the tab strip; double-clicking a name starts renaming it. */
function renderTabs() {
  tabList.replaceChildren(...tabs.map((tab, index) => {
    const el = document.createElement('div');
    el.className = index === activeTab ? 'tab active' : 'tab';
    const label = document.createElement('span');
    label.className = 'tab-name';
    label.textContent = tab.name;
    label.title = 'Double-click to rename';
    label.addEventListener('dblclick', () => startRename(index, label));
    el.addEventListener('click', () => switchTab(index));
    el.append(label);
    return el;
  }));
}

/** Makes a different tab active, restoring its texts and comparing them. */
function switchTab(index) {
  if (index === activeTab) return;
  saveActiveTab();
  activeTab = index;
  leftInput.value = tabs[index].left;
  rightInput.value = tabs[index].right;
  renderTabs();
  compare();
}

/** Opens a new empty tab and makes it active. */
function addTab() {
  saveActiveTab();
  tabs.push({ name: `Tab ${tabs.length + 1}`, left: '', right: '' });
  switchTab(tabs.length - 1);
}

/** Replaces a tab name label with a text field; Enter or blur saves, Escape cancels. */
function startRename(index, label) {
  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'tab-rename';
  input.value = tabs[index].name;
  let done = false;
  const finish = (save) => {
    if (done) return;
    done = true;
    const name = input.value.trim();
    if (save && name) tabs[index].name = name;
    renderTabs();
  };
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') finish(true);
    if (event.key === 'Escape') finish(false);
  });
  input.addEventListener('blur', () => finish(true));
  label.replaceWith(input);
  input.focus();
  input.select();
}

/** Compares the active tab's texts and renders the result. */
async function compare() {
  saveActiveTab();
  const { left, right } = tabs[activeTab];
  if (!left && !right) {
    clear();
    status.textContent = PROMPT;
    return;
  }
  const view = await invoke('compare_text', { left, right });
  render(view);
}

/** Renders a comparison view as aligned side-by-side rows. */
function render(view) {
  clear();
  for (const block of view.blocks) {
    const count = Math.max(block.left.length, block.right.length);
    for (let i = 0; i < count; i++) {
      const row = document.createElement('tr');
      row.className = block.kind;
      row.append(cell(block.left[i], 'left'), cell(block.right[i], 'right'));
      rows.append(row);
    }
  }
  status.textContent = view.identical
    ? 'Files are identical.'
    : `${view.difference_count} difference(s).`;
}

/** Builds one table cell for a line, or an empty cell when the side has no line. */
function cell(line, side) {
  const td = document.createElement('td');
  td.className = line ? side : `${side} empty`;
  if (line) {
    const number = document.createElement('span');
    number.className = 'no';
    number.textContent = line.number;
    td.append(number);
    for (const segment of line.segments) {
      if (segment.changed) {
        const mark = document.createElement('mark');
        mark.textContent = segment.text;
        td.append(mark);
      } else {
        td.append(segment.text);
      }
    }
  }
  return td;
}

/** Clears the comparison rows. */
function clear() {
  rows.replaceChildren();
}

/** Toggles line wrapping in the comparison table to match the Wrap checkbox. */
function applyWrap() {
  document.body.classList.toggle('wrap', wrapToggle.checked);
}

leftInput.addEventListener('input', compare);
rightInput.addEventListener('input', compare);
addTabButton.addEventListener('click', addTab);
wrapToggle.addEventListener('change', applyWrap);
applyWrap();
renderTabs();
