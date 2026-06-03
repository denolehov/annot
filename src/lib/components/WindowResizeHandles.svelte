<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  // ResizeDirection is declared in @tauri-apps/api/window.d.ts but not exported, so inline it here.
  type ResizeDirection =
    | 'East'
    | 'North'
    | 'NorthEast'
    | 'NorthWest'
    | 'South'
    | 'SouthEast'
    | 'SouthWest'
    | 'West';

  function onMouseDown(direction: ResizeDirection) {
    return (e: MouseEvent) => {
      e.preventDefault();
      getCurrentWindow().startResizeDragging(direction);
    };
  }
</script>

{#if !__IS_MACOS__}
  <!-- Edges first so the corner handles (rendered after) win clicks at the overlap. -->
  <div class="handle edge n"  onmousedown={onMouseDown('North')} role="presentation"></div>
  <div class="handle edge s"  onmousedown={onMouseDown('South')} role="presentation"></div>
  <div class="handle edge w"  onmousedown={onMouseDown('West')}  role="presentation"></div>
  <div class="handle edge e"  onmousedown={onMouseDown('East')}  role="presentation"></div>
  <!-- Corners (larger hit areas, must stack on top of edges) -->
  <div class="handle corner nw" onmousedown={onMouseDown('NorthWest')} role="presentation"></div>
  <div class="handle corner ne" onmousedown={onMouseDown('NorthEast')} role="presentation"></div>
  <div class="handle corner sw" onmousedown={onMouseDown('SouthWest')} role="presentation"></div>
  <div class="handle corner se" onmousedown={onMouseDown('SouthEast')} role="presentation"></div>
{/if}

<style>
  .handle {
    position: fixed;
    z-index: 9999;
  }

  /* Edges */
  .edge.n { top: 0;    left: 0;    right: 0;    height: 5px; cursor: n-resize;  }
  .edge.s { bottom: 0; left: 0;    right: 0;    height: 5px; cursor: s-resize;  }
  .edge.w { left: 0;   top: 0;     bottom: 0;   width: 5px;  cursor: w-resize;  }
  .edge.e { right: 0;  top: 0;     bottom: 0;   width: 5px;  cursor: e-resize;  }

  /* Corners (12px, must stack above edges so diagonal grabs win in the overlap) */
  .corner { z-index: 10000; }
  .corner.nw { top: 0;    left: 0;   width: 12px; height: 12px; cursor: nw-resize; }
  .corner.ne { top: 0;    right: 0;  width: 12px; height: 12px; cursor: ne-resize; }
  .corner.sw { bottom: 0; left: 0;   width: 12px; height: 12px; cursor: sw-resize; }
  .corner.se { bottom: 0; right: 0;  width: 12px; height: 12px; cursor: se-resize; }
</style>
