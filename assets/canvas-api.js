/**
 * Canvas Drawing API for ROI visualization
 * This module handles all canvas drawing operations
 */

window.CanvasDrawAPI = {
  /**
   * Redraw the canvas with all ROIs and current drawing state
   * @param {Object} config - Configuration object
   * @param {HTMLCanvasElement} config.canvas - Canvas element
   * @param {number} config.scale - Zoom scale factor
   * @param {number} config.offset_x - X offset for panning
   * @param {number} config.offset_y - Y offset for panning
   * @param {HTMLImageElement|null} config.image - Background image
   * @param {Array<Array<[number, number]>>} config.drawed_rois - Completed ROIs
   * @param {Array<[number, number]>} config.current_points - Current drawing points
   * @param {[number, number]|null} config.mouse_xy - Current mouse position in canvas coordinates
   * @param {Object|null} config.highlight - Highlight info {name, is_edit}
   */
  redraw(config) {
    const {
      scale,
      offset_x,
      offset_y,
      drawed_rois,
      current_points,
      mouse_xy,
      highlight,
    } = config;

    // Find canvas element
    const canvas = document.getElementById("roi-canvas");
    if (!canvas) {
      console.warn("Canvas element 'roi-canvas' not found");
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      console.warn("Failed to get 2D context from canvas");
      return;
    }

    // Find image element
    const imageEl = document.getElementById("roi-image");
    const imageData = imageEl && imageEl.src ? imageEl.src : null;

    // Clear and reset
    ctx.resetTransform();
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Apply transformations
    ctx.scale(scale, scale);

    // Draw background image
    if (imageData) {
      const img = new Image();
      img.onload = () => {
        ctx.drawImage(img, offset_x, offset_y);
        drawROIs();
      };
      img.onerror = () => {
        console.warn("Failed to load image:", imageData);
        drawROIs();
      };
      img.src = imageData;
    } else {
      drawROIs();
    }

    function drawROIs() {
      const offsetSize = { x: offset_x, y: offset_y };

      // Draw completed ROIs (green)
      ctx.strokeStyle = "green";
      ctx.fillStyle = "green";
      ctx.lineWidth = 2.0;

      if (drawed_rois && Array.isArray(drawed_rois)) {
        drawed_rois.forEach((roi, index) => {
          // Skip if this is the highlighted ROI (draw it later)
          if (highlight && highlight.name === `ROI ${index}`) {
            return;
          }

          if (Array.isArray(roi) && roi.length > 0) {
            ctx.beginPath();
            roi.forEach((point, i) => {
              const [x, y] = point;
              const canvasX = x + offsetSize.x;
              const canvasY = y + offsetSize.y;
              if (i === 0) {
                ctx.moveTo(canvasX, canvasY);
              } else {
                ctx.lineTo(canvasX, canvasY);
              }
            });
            ctx.closePath();
            ctx.stroke();

            // Draw vertices (small circles)
            roi.forEach((point) => {
              const [x, y] = point;
              const canvasX = x + offsetSize.x;
              const canvasY = y + offsetSize.y;
              ctx.beginPath();
              ctx.arc(canvasX, canvasY, 4.0, 0, Math.PI * 2);
              ctx.fill();
            });
          }
        });
      }

      // Draw current polygon (red)
      ctx.strokeStyle = "red";
      ctx.fillStyle = "red";
      ctx.lineWidth = 2.0;

      if (current_points && Array.isArray(current_points)) {
        ctx.beginPath();
        current_points.forEach((point, i) => {
          const [x, y] = point;
          const canvasX = x + offsetSize.x;
          const canvasY = y + offsetSize.y;
          if (i === 0) {
            ctx.moveTo(canvasX, canvasY);
          } else {
            ctx.lineTo(canvasX, canvasY);
          }
        });

        // Draw preview line to mouse
        if (mouse_xy) {
          ctx.lineTo(mouse_xy[0], mouse_xy[1]);
        }
        ctx.stroke();

        // Draw current vertices (larger circles)
        current_points.forEach((point) => {
          const [x, y] = point;
          const canvasX = x + offsetSize.x;
          const canvasY = y + offsetSize.y;
          ctx.beginPath();
          ctx.arc(canvasX, canvasY, 5.0, 0, Math.PI * 2);
          ctx.fill();
        });
      }

      // Draw highlighted ROI (red, thicker)
      if (highlight && highlight.name) {
        const roiIndex = parseInt(highlight.name.replace("ROI ", ""));
        const targetROI =
          drawed_rois && Array.isArray(drawed_rois) ? drawed_rois[roiIndex] : null;

        if (targetROI && Array.isArray(targetROI)) {
          ctx.strokeStyle = "red";
          ctx.fillStyle = "red";
          ctx.lineWidth = 4.0;

          ctx.beginPath();
          targetROI.forEach((point, i) => {
            const [x, y] = point;
            const canvasX = x + offsetSize.x;
            const canvasY = y + offsetSize.y;
            if (i === 0) {
              ctx.moveTo(canvasX, canvasY);
            } else {
              ctx.lineTo(canvasX, canvasY);
            }
          });
          ctx.closePath();
          ctx.stroke();

          // Draw highlighted vertices (larger)
          targetROI.forEach((point) => {
            const [x, y] = point;
            const canvasX = x + offsetSize.x;
            const canvasY = y + offsetSize.y;
            ctx.beginPath();
            ctx.arc(canvasX, canvasY, 6.0, 0, Math.PI * 2);
            ctx.fill();
          });
        }
      }

      // Draw mouse crosshair
      ctx.strokeStyle = "black";
      ctx.fillStyle = "black";
      if (mouse_xy) {
        ctx.lineWidth = 1.0;
        ctx.beginPath();
        ctx.moveTo(mouse_xy[0] - 20, mouse_xy[1]);
        ctx.lineTo(mouse_xy[0] + 20, mouse_xy[1]);
        ctx.moveTo(mouse_xy[0], mouse_xy[1] - 20);
        ctx.lineTo(mouse_xy[0], mouse_xy[1] + 20);
        ctx.stroke();
      }
    }
  },
};

// Export as global for wasm-bindgen access
window.drawROICanvas = function (config) {
  const apiConfig = {
    ...config,
    // Image will be loaded from DOM element in the redraw function
  };
  window.CanvasDrawAPI.redraw(apiConfig);
};
