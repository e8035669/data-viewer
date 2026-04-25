// assets/roi.js
const roiHandlerProto = {
    image: null,
    /** @type {HTMLCanvasElement} */
    canvas: null,
    /** @type {CanvasRenderingContext2D} */
    ctx: null,

    /** @type {HTMLCanvasElement} */
    magnifier_canvas: null,
    /** @type {CanvasRenderingContext2D} */
    magnifier_ctx: null,

    scale: 1.0,
    offset: { x: 0, y: 0 },
    mouse: null,
    current_points: [],
    drawed_rois: new Map(),
    highlight: null,

    init(imageId, canvasId, magnifierId) {
        const image = document.getElementById(imageId);
        this.image = image;
        const canvas = document.getElementById(canvasId);
        this.canvas = canvas;
        if (canvas) {
            this.ctx = canvas.getContext('2d');
        }
        const magnifier_canvas = document.getElementById(magnifierId);
        this.magnifier_canvas = magnifier_canvas;
        if (magnifier_canvas) {
            this.magnifier_ctx = magnifier_canvas.getContext('2d');
        }
        this.offset = { x: 0, y: 0 };
        this.mouse = null;
        this.drawed_rois = new Map();
        this.highlight = null;
        console.info("init", image, canvas, magnifier_canvas);
        return !!(image && canvas && this.ctx && magnifier_canvas);
    },

    setScale(scale) {
        this.scale = scale;
    },

    setOffset(x, y) {
        this.offset = { x, y };
    },

    setMouse(x, y) {
        this.mouse = { x, y };
    },

    clearMouse() {
        this.mouse = null;
    },

    setCurrentPoints(points) {
        this.current_points = [];
        for (let i = 0; i < points.length; i++) {
            let p = points[i];
            this.current_points.push({ x: p[0], y: p[1] });
        }
    },

    clearCurrentPoints() {
        this.current_points = [];
    },

    addPoint(x, y) {
        if (!this.ctx) return;
        this.current_points.push({ x, y });
        this.redraw();
    },

    addDrawedRoi(name, points) {
        let tmp = [];
        for (let i = 0; i < points.length; i++) {
            let p = points[i];
            tmp.push({ x: p[0], y: p[1] });
        }
        this.drawed_rois.set(name, tmp);
    },

    removeDrawedRoi(name) {
        this.drawed_rois.delete(name);
    },

    clearDrawedRoi() {
        this.drawed_rois.clear();
    },

    setHighlight(name) {
        this.highlight = name;
    },

    clearHighlight() {
        this.highlight = null;
    },

    redraw() {
        const { ctx, canvas } = this;
        ctx.resetTransform();
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        ctx.scale(this.scale, this.scale);

        if (this.image) {
            ctx.drawImage(this.image, this.offset.x, this.offset.y);
        }

        ctx.strokeStyle = "green";
        ctx.fillStyle = "green";
        ctx.lineWidth = 2.0;
        for (const [key, value] of this.drawed_rois) {
            ctx.beginPath();
            for (let i = 0; i < value.length; i++) {
                let p = value[i];
                let x = p.x + this.offset.x;
                let y = p.y + this.offset.y;
                if (i === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            }
            ctx.closePath();
            ctx.stroke();

            for (let i = 0; i < value.length; i++) {
                let p = value[i];
                let x = p.x + this.offset.x;
                let y = p.y + this.offset.y;
                ctx.beginPath();
                ctx.arc(x, y, 4.0, 0.0, Math.PI * 2);
                ctx.fill();
            }
        }

        ctx.strokeStyle = "red";
        ctx.fillStyle = "red";
        ctx.lineWidth = 2.0;
        if (this.current_points.length !== 0) {
            ctx.beginPath()

            for (let i = 0; i < this.current_points.length; i++) {
                let p = this.current_points[i];
                let x = p.x + this.offset.x;
                let y = p.y + this.offset.y;
                if (i === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            }

            if (this.mouse) {
                ctx.lineTo(this.mouse.x, this.mouse.y);
            }
            ctx.stroke()

            for (let i = 0; i < this.current_points.length; i++) {
                let p = this.current_points[i];
                let x = p.x + this.offset.x;
                let y = p.y + this.offset.y;
                ctx.beginPath();
                ctx.arc(x, y, 5.0, 0.0, Math.PI * 2);
                ctx.fill();
            }
        }

        if (this.highlight) {
            let value = this.drawed_rois.get(this.highlight);
            if (value) {
                ctx.strokeStyle = "red";
                ctx.fillStyle = "red";
                ctx.lineWidth = 4.0;

                ctx.beginPath();
                for (let i = 0; i < value.length; i++) {
                    let p = value[i];
                    let x = p.x + this.offset.x;
                    let y = p.y + this.offset.y;
                    if (i === 0) {
                        ctx.moveTo(x, y);
                    } else {
                        ctx.lineTo(x, y);
                    }
                }
                ctx.closePath();
                ctx.stroke();

                for (let i = 0; i < value.length; i++) {
                    let p = value[i];
                    let x = p.x + this.offset.x;
                    let y = p.y + this.offset.y;
                    ctx.beginPath();
                    ctx.arc(x, y, 6.0, 0.0, Math.PI * 2);
                    ctx.fill();
                }
            }
        }

        ctx.strokeStyle = "black";
        ctx.fillStyle = "black";
        ctx.lineWidth = 1.0;
        if (this.mouse) {
            const m = this.mouse;
            ctx.beginPath();
            ctx.moveTo(m.x - 20, m.y);
            ctx.lineTo(m.x + 20, m.y);
            ctx.moveTo(m.x, m.y - 20);
            ctx.lineTo(m.x, m.y + 20);
            ctx.stroke();
        }

        this.handle_magnifier_pos();
        this.draw_magnifier();
    },

    handle_magnifier_pos() {
        const canvas = this.magnifier_canvas;
        if (this.mouse) {
            canvas.classList.remove("hidden");

            const mouse_x = this.mouse.x;
            const canvas_width = this.canvas.width / this.scale;

            if (mouse_x > canvas_width / 2) {
                canvas.classList.remove("right-0");
                canvas.classList.add("left-0");
            } else {
                canvas.classList.remove("left-0");
                canvas.classList.add("right-0");
            }

        } else {
            canvas.classList.add("hidden");
        }
    },

    draw_magnifier() {
        const ctx = this.magnifier_ctx;
        const canvas = this.magnifier_canvas;
        ctx.resetTransform();
        ctx.clearRect(0, 0, canvas.width, canvas.height);


        if (this.mouse && this.image) {
            const m = this.mouse;
            // ctx.translate(-m.x + this.offset.x + 100, -m.y + this.offset.y + 100);
            const center_x = m.x - this.offset.x;
            const center_y = m.y - this.offset.y;
            const range = 20;
            ctx.drawImage(this.image, center_x - range, center_y - range, range * 2, range * 2, 0, 0, canvas.width, canvas.height);
        }

        ctx.resetTransform();
        ctx.beginPath();
        ctx.moveTo(canvas.width / 2, 0);
        ctx.lineTo(canvas.width / 2, canvas.height);
        ctx.moveTo(0, canvas.height / 2);
        ctx.lineTo(canvas.width, canvas.height / 2);
        ctx.stroke();
    },

    helloworld() {
        console.log("Hello World!");
    }

};

window.roiHandlerProto = roiHandlerProto;

// 由rust端建立物件
// window.roiHandler = Object.create(roiHandlerProto);
// console.log("Create roiHandler", window.roiHandler)
