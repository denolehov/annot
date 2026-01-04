import { describe, it, expect, vi } from 'vitest';
import { useHistory, emptySessionData, type SessionData } from './useHistory.svelte';

describe('useHistory', () => {
  it('starts with empty session data', () => {
    const history = useHistory();
    expect(history.current.annotations).toEqual({});
    expect(history.current.terraform).toEqual([]);
    expect(history.current.sessionComment).toBeNull();
    expect(history.current.selectedExitMode).toBeNull();
  });

  it('can push new state', () => {
    const history = useHistory();
    const newState: SessionData = {
      ...emptySessionData(),
      annotations: {
        '10-15': {
          range: { start: 10, end: 15 },
          content: { type: 'doc', content: [] },
        },
      },
    };

    history.push(newState, 'Created annotation');

    expect(history.current.annotations['10-15']).toBeDefined();
    expect(history.getHistoryLength()).toBe(2);
  });

  it('can undo and redo', () => {
    const history = useHistory();

    // Push first change
    history.push({
      ...emptySessionData(),
      annotations: { '10-15': { range: { start: 10, end: 15 }, content: { type: 'doc' } } },
    }, 'First');

    // Push second change
    history.push({
      ...emptySessionData(),
      annotations: {
        '10-15': { range: { start: 10, end: 15 }, content: { type: 'doc' } },
        '20-25': { range: { start: 20, end: 25 }, content: { type: 'doc' } },
      },
    }, 'Second');

    expect(Object.keys(history.current.annotations)).toHaveLength(2);

    // Undo
    expect(history.canUndo()).toBe(true);
    history.undo();
    expect(Object.keys(history.current.annotations)).toHaveLength(1);

    // Undo again
    history.undo();
    expect(Object.keys(history.current.annotations)).toHaveLength(0);

    // Can't undo anymore
    expect(history.canUndo()).toBe(false);
    expect(history.undo()).toBe(false);

    // Redo
    expect(history.canRedo()).toBe(true);
    history.redo();
    expect(Object.keys(history.current.annotations)).toHaveLength(1);

    history.redo();
    expect(Object.keys(history.current.annotations)).toHaveLength(2);

    // Can't redo anymore
    expect(history.canRedo()).toBe(false);
    expect(history.redo()).toBe(false);
  });

  it('truncates future when pushing after undo', () => {
    const history = useHistory();

    history.push({ ...emptySessionData(), selectedExitMode: 'a' }, 'First');
    history.push({ ...emptySessionData(), selectedExitMode: 'b' }, 'Second');
    history.push({ ...emptySessionData(), selectedExitMode: 'c' }, 'Third');

    expect(history.getHistoryLength()).toBe(4); // initial + 3

    // Undo twice
    history.undo();
    history.undo();
    expect(history.current.selectedExitMode).toBe('a');

    // Push new state - should truncate 'b' and 'c'
    history.push({ ...emptySessionData(), selectedExitMode: 'd' }, 'New branch');

    expect(history.getHistoryLength()).toBe(3); // initial + 'a' + 'd'
    expect(history.current.selectedExitMode).toBe('d');
    expect(history.canRedo()).toBe(false);
  });

  it('calls onStateChange callback', () => {
    const onStateChange = vi.fn();
    const history = useHistory({ onStateChange });

    history.push({ ...emptySessionData(), selectedExitMode: 'a' }, 'Test');
    expect(onStateChange).toHaveBeenCalledTimes(1);
    expect(onStateChange).toHaveBeenCalledWith(
      expect.objectContaining({ selectedExitMode: 'a' }),
      'Test'
    );

    history.undo();
    expect(onStateChange).toHaveBeenCalledTimes(2);
    expect(onStateChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ selectedExitMode: null }),
      'Undo'
    );
  });

  it('generates homerow IDs for narrative', () => {
    const history = useHistory();

    history.push({ ...emptySessionData(), selectedExitMode: 'a' }, 'First');
    history.push({ ...emptySessionData(), selectedExitMode: 'b' }, 'Second');
    history.push({ ...emptySessionData(), selectedExitMode: 'c' }, 'Third');

    const narrative = history.getNarrative();
    expect(narrative).toHaveLength(3);
    expect(narrative[0].id).toBe('aa');
    expect(narrative[1].id).toBe('as');
    expect(narrative[2].id).toBe('ad');
  });

  it('clones state to ensure immutability', () => {
    const history = useHistory();

    const original: SessionData = {
      ...emptySessionData(),
      annotations: {
        '10-15': { range: { start: 10, end: 15 }, content: { type: 'doc' } },
      },
    };

    history.push(original, 'Push');

    // Modify original
    original.annotations['10-15'].range.start = 999;

    // History should not be affected
    expect(history.current.annotations['10-15'].range.start).toBe(10);
  });

  it('initialize resets history', () => {
    const history = useHistory();

    history.push({ ...emptySessionData(), selectedExitMode: 'a' }, 'First');
    history.push({ ...emptySessionData(), selectedExitMode: 'b' }, 'Second');

    const initialState: SessionData = {
      ...emptySessionData(),
      selectedExitMode: 'initial',
    };

    history.initialize(initialState);

    expect(history.getHistoryLength()).toBe(1);
    expect(history.current.selectedExitMode).toBe('initial');
    expect(history.canUndo()).toBe(false);
    expect(history.getNarrative()).toHaveLength(0);
  });
});
