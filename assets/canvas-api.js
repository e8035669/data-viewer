// assets/roi.js
const roiHandlerProto = {
    image: null,
    canvas: null,
    /** @type {CanvasRenderingContext2D} */
    ctx: null,

    scale: 1.0,
    offset: {x: 0, y: 0},
    mouse: null,
    points: [],

    init(imageId, canvasId) {
        const image = document.getElementById(imageId);
        this.image = image;
        const canvas = document.getElementById(canvasId);
        this.canvas = canvas;
        if (canvas) {
            this.ctx = canvas.getContext('2d');
        }
        this.offset = {x: 0, y: 0};
        this.mouse = null;
    },

    setScale(scale) {
        this.scale = scale;
    },

    setOffset(x, y) {
        this.offset = {x, y};
    },

    setMouse(x, y) {
        this.mouse = {x, y};
    },

    clearMouse() {
        this.mouse = null;
    },

    addPoint(x, y) {
        if (!this.ctx) return;
        this.points.push({ x, y });
        this.redraw();
    },

    redraw() {
        const { ctx } = this;
        ctx.reset();
        ctx.scale(this.scale, this.scale);

        if (this.image) {
            ctx.drawImage(this.image, this.offset.x, this.offset.y);
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
    },

    helloworld() {
        console.log("Hello World!");
    }

};

window.roiHandler = Object.create(roiHandlerProto);
