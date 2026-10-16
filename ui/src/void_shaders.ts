/**
 * Void Shaders Engine (Plainpad / Antinote 1:1 Parity)
 *
 * Implements GPU fragment shaders matching Antinote's default.metallib:
 * 1. voidDistortionSlow: Ambient gravitational warping on The Void canvas
 * 2. voidRipple: Wave distortion triggering on note soft-deletion
 * 3. voidDistortionBurst: High-energy explosion triggering on Empty The Void
 *
 * Includes procedural Canvas 2D fallback for headless/non-WebGL environments.
 */

interface RippleState {
  startTime: number;
  duration: number; // seconds
  originX: number;
  originY: number;
  color: [number, number, number, number];
  onComplete?: () => void;
}

interface BurstState {
  startTime: number;
  duration: number; // seconds
  timeOffset: number;
  onComplete?: () => void;
}

const VS_QUAD = `
attribute vec2 a_position;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const FS_VOID_DISTORTION_SLOW = `
precision mediump float;
uniform float u_time;
uniform float u_time_offset;
uniform vec2 u_resolution;
uniform vec2 u_center;

void main() {
  vec2 coord = gl_FragCoord.xy;
  vec2 st = (coord - u_center) / min(u_resolution.x, u_resolution.y);
  float r = length(st);
  float theta = atan(st.y, st.x);

  float t = u_time * 0.5 + u_time_offset;
  float twist = (1.0 / (r + 0.2)) * sin(t);
  theta += twist;

  // Swirling gravitational filaments
  float spiral = sin(theta * 4.0 - r * 10.0 + t * 0.7) * 0.5 + 0.5;
  float filament = pow(spiral, 3.0) * exp(-r * 2.5);

  // Subtle luminous haze around black hole
  float haze = exp(-r * 3.5) * 0.5;

  // Core event horizon (darkness at the center)
  float core = smoothstep(0.06, 0.24, r);

  vec3 deepSpace = vec3(0.05, 0.03, 0.10);
  vec3 filamentCol = vec3(0.60, 0.28, 0.92);
  vec3 glowCol = vec3(0.25, 0.50, 0.95);

  vec3 rgb = (deepSpace + filament * filamentCol + haze * glowCol) * core;
  float alpha = clamp((filament * 0.75 + haze * 0.55 + (1.0 - core) * 0.35), 0.0, 0.82);

  gl_FragColor = vec4(rgb, alpha);
}
`;

const FS_VOID_RIPPLE = `
precision mediump float;
uniform float u_time;
uniform vec2 u_resolution;
uniform vec2 u_center;
uniform vec4 u_color;

void main() {
  vec2 coord = gl_FragCoord.xy;
  vec2 st = (coord - u_center) / min(u_resolution.x, u_resolution.y);
  float dist = length(st);

  float duration = 1.2;
  float speed = 2.0;
  float freq = 28.0;
  float decay = 3.2;

  float progress = clamp(u_time / duration, 0.0, 1.0);
  float fade = max(0.0, 1.0 - progress);

  // Concentric sinusoidal ripple & shockwave ring
  float wave = sin(dist * freq - u_time * speed * 8.0) * exp(-dist * decay) * fade;
  float ringDist = abs(dist - progress * 1.6);
  float ring = exp(-ringDist * 25.0) * fade;

  float intensity = clamp(abs(wave) * 0.7 + ring * 1.1, 0.0, 1.0);
  vec3 rgb = mix(u_color.rgb, vec3(0.8, 0.6, 1.0), ring * 0.7);
  float alpha = intensity * u_color.a * fade;

  gl_FragColor = vec4(rgb, alpha);
}
`;

const FS_VOID_DISTORTION_BURST = `
precision mediump float;
uniform float u_time;
uniform float u_time_offset;
uniform vec2 u_resolution;
uniform vec2 u_center;

void main() {
  vec2 coord = gl_FragCoord.xy;
  vec2 st = (coord - u_center) / min(u_resolution.x, u_resolution.y);
  float r = length(st);
  float theta = atan(st.y, st.x);

  float duration = 1.5;
  float progress = clamp(u_time / duration, 0.0, 1.0);
  float fade = max(0.0, 1.0 - progress);

  // Expanding high-energy shockwave ring
  float ringRadius = progress * 2.2;
  float shock = exp(-abs(r - ringRadius) * 20.0) * smoothstep(1.0, 0.2, progress);

  // Chaotic noise/energy filaments
  float noise = abs(sin(theta * 8.0 + r * 16.0 - u_time * 12.0) * sin(theta * 4.0 + u_time * 9.0));
  float energy = noise * exp(-r * (2.0 + progress * 5.0)) * fade;

  // Inward gravitational collapse
  float collapse = exp(-r * (4.0 + progress * 6.0)) * smoothstep(0.2, 0.8, progress) * fade * 1.4;

  vec3 burstColor = mix(vec3(1.0, 0.45, 0.2), vec3(0.65, 0.25, 0.98), progress);
  vec3 coreColor = vec3(1.0, 0.95, 0.9);

  vec3 rgb = mix(burstColor, coreColor, clamp(shock * 1.2 + collapse, 0.0, 1.0));
  float alpha = clamp(shock * 1.3 + energy * 0.7 + collapse * 1.2, 0.0, 1.0) * fade;

  gl_FragColor = vec4(rgb, alpha);
}
`;

export class VoidShaderController {
  private canvas: HTMLCanvasElement | null = null;
  private gl: WebGLRenderingContext | WebGL2RenderingContext | null = null;
  private ctx2d: CanvasRenderingContext2D | null = null;
  private animFrameId: number | null = null;

  private isAmbientRunning = false;
  private ambientStartTime = 0;
  private ambientTimeOffset = 0;

  private ripple: RippleState | null = null;
  private burst: BurstState | null = null;

  // WebGL Buffers & Programs
  private quadBuffer: WebGLBuffer | null = null;
  private progAmbient: WebGLProgram | null = null;
  private progRipple: WebGLProgram | null = null;
  private progBurst: WebGLProgram | null = null;

  // Uniform locations: Ambient
  private uAmbientTime: WebGLUniformLocation | null = null;
  private uAmbientOffset: WebGLUniformLocation | null = null;
  private uAmbientRes: WebGLUniformLocation | null = null;
  private uAmbientCenter: WebGLUniformLocation | null = null;

  // Uniform locations: Ripple
  private uRippleTime: WebGLUniformLocation | null = null;
  private uRippleRes: WebGLUniformLocation | null = null;
  private uRippleCenter: WebGLUniformLocation | null = null;
  private uRippleColor: WebGLUniformLocation | null = null;

  // Uniform locations: Burst
  private uBurstTime: WebGLUniformLocation | null = null;
  private uBurstOffset: WebGLUniformLocation | null = null;
  private uBurstRes: WebGLUniformLocation | null = null;
  private uBurstCenter: WebGLUniformLocation | null = null;

  private resizeHandler = () => this.resize();

  /**
   * Attach to full-bleed canvas element and initialize WebGL or Canvas 2D fallback.
   */
  public attach(canvas: HTMLCanvasElement): void {
    this.canvas = canvas;
    window.addEventListener('resize', this.resizeHandler);
    this.resize();

    try {
      const gl = canvas.getContext('webgl2', { alpha: true, antialias: true }) ||
                 canvas.getContext('webgl', { alpha: true, antialias: true });
      if (gl) {
        this.gl = gl as WebGLRenderingContext;
        this.initWebGL();
      } else {
        this.initFallback2D();
      }
    } catch {
      this.initFallback2D();
    }
  }

  /**
   * Disconnect and release resources.
   */
  public detach(): void {
    this.stopAmbient();
    this.stopLoop();
    window.removeEventListener('resize', this.resizeHandler);
    this.canvas = null;
    this.gl = null;
    this.ctx2d = null;
  }

  /**
   * Starts ambient slow gravitational warping (voidDistortionSlow) at steady 60fps.
   */
  public startAmbient(): void {
    this.isAmbientRunning = true;
    this.ambientStartTime = performance.now();
    this.ambientTimeOffset = Math.random() * 20.0;
    this.showCanvas();
    this.ensureLoopRunning();
  }

  /**
   * Stops ambient rendering to conserve GPU and CPU cycles when Void view is closed.
   */
  public stopAmbient(): void {
    this.isAmbientRunning = false;
    if (!this.ripple && !this.burst) {
      this.stopLoop();
      this.hideCanvas();
    }
  }

  /**
   * Triggers expanding concentric wave distortion (voidRipple) on note soft-deletion.
   */
  public triggerRipple(originX?: number, originY?: number, onComplete?: () => void): void {
    if (!this.canvas) return;
    const rect = this.canvas.getBoundingClientRect();
    const x = originX !== undefined ? originX : (rect.left + rect.width * 0.5);
    const y = originY !== undefined ? originY : (rect.top + rect.height * 0.5);

    this.ripple = {
      startTime: performance.now(),
      duration: 1.2,
      originX: x,
      originY: y,
      color: [0.55, 0.35, 0.95, 0.9],
      onComplete,
    };

    this.showCanvas();
    this.ensureLoopRunning();
  }

  /**
   * Triggers high-energy shockwave explosion (voidDistortionBurst) on Empty The Void.
   */
  public triggerBurst(onComplete?: () => void): void {
    if (!this.canvas) return;

    this.burst = {
      startTime: performance.now(),
      duration: 1.5,
      timeOffset: Math.random() * 10.0,
      onComplete,
    };

    this.showCanvas();
    this.ensureLoopRunning();
  }

  /**
   * Resizes canvas internal pixel buffer to match display rect.
   */
  public resize(): void {
    if (!this.canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.floor((this.canvas.clientWidth || window.innerWidth) * dpr));
    const height = Math.max(1, Math.floor((this.canvas.clientHeight || window.innerHeight) * dpr));

    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      if (this.gl) {
        this.gl.viewport(0, 0, width, height);
      }
    }
  }

  // ---------- WebGL Pipeline Setup ----------

  private initWebGL(): void {
    const gl = this.gl;
    if (!gl) return;

    // Compile Shader Programs
    this.progAmbient = this.createProgram(gl, VS_QUAD, FS_VOID_DISTORTION_SLOW);
    this.progRipple = this.createProgram(gl, VS_QUAD, FS_VOID_RIPPLE);
    this.progBurst = this.createProgram(gl, VS_QUAD, FS_VOID_DISTORTION_BURST);

    if (!this.progAmbient || !this.progRipple || !this.progBurst) {
      throw new Error('Failed to compile Void fragment shader programs');
    }

    // Cache Uniform Locations
    this.uAmbientTime = gl.getUniformLocation(this.progAmbient, 'u_time');
    this.uAmbientOffset = gl.getUniformLocation(this.progAmbient, 'u_time_offset');
    this.uAmbientRes = gl.getUniformLocation(this.progAmbient, 'u_resolution');
    this.uAmbientCenter = gl.getUniformLocation(this.progAmbient, 'u_center');

    this.uRippleTime = gl.getUniformLocation(this.progRipple, 'u_time');
    this.uRippleRes = gl.getUniformLocation(this.progRipple, 'u_resolution');
    this.uRippleCenter = gl.getUniformLocation(this.progRipple, 'u_center');
    this.uRippleColor = gl.getUniformLocation(this.progRipple, 'u_color');

    this.uBurstTime = gl.getUniformLocation(this.progBurst, 'u_time');
    this.uBurstOffset = gl.getUniformLocation(this.progBurst, 'u_time_offset');
    this.uBurstRes = gl.getUniformLocation(this.progBurst, 'u_resolution');
    this.uBurstCenter = gl.getUniformLocation(this.progBurst, 'u_center');

    // Create Screen Quad Buffer
    const quadVertices = new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
      -1,  1,
       1, -1,
       1,  1,
    ]);
    this.quadBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, quadVertices, gl.STATIC_DRAW);

    // Blending Configuration
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  }

  private initFallback2D(): void {
    if (!this.canvas) return;
    this.ctx2d = this.canvas.getContext('2d');
  }

  private createShader(gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null {
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Shader compilation error:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  private createProgram(gl: WebGLRenderingContext, vsSource: string, fsSource: string): WebGLProgram | null {
    const vs = this.createShader(gl, gl.VERTEX_SHADER, vsSource);
    const fs = this.createShader(gl, gl.FRAGMENT_SHADER, fsSource);
    if (!vs || !fs) return null;

    const prog = gl.createProgram();
    if (!prog) return null;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);

    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.error('Program link error:', gl.getProgramInfoLog(prog));
      gl.deleteProgram(prog);
      return null;
    }
    return prog;
  }

  // ---------- Render Loop & State ----------

  private ensureLoopRunning(): void {
    if (this.animFrameId === null) {
      this.animFrameId = requestAnimationFrame(this.renderLoop);
    }
  }

  private stopLoop(): void {
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
    this.clear();
  }

  private showCanvas(): void {
    if (this.canvas && this.canvas.classList.contains('hidden')) {
      this.canvas.classList.remove('hidden');
    }
  }

  private hideCanvas(): void {
    if (this.canvas && !this.canvas.classList.contains('hidden')) {
      this.canvas.classList.add('hidden');
    }
  }

  private renderLoop = (timestamp: number) => {
    if (!this.canvas) {
      this.animFrameId = null;
      return;
    }

    if (!this.isAmbientRunning && !this.ripple && !this.burst) {
      this.stopLoop();
      this.hideCanvas();
      return;
    }

    this.resize();

    if (this.gl) {
      this.renderWebGL(timestamp);
    } else if (this.ctx2d) {
      this.renderFallback2D(timestamp);
    }

    this.animFrameId = requestAnimationFrame(this.renderLoop);
  };

  private clear(): void {
    if (this.gl) {
      this.gl.clearColor(0, 0, 0, 0);
      this.gl.clear(this.gl.COLOR_BUFFER_BIT);
    } else if (this.ctx2d && this.canvas) {
      this.ctx2d.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  // ---------- WebGL Frame Render ----------

  private renderWebGL(timestamp: number): void {
    const gl = this.gl;
    if (!gl || !this.canvas) return;

    gl.clearColor(0.0, 0.0, 0.0, 0.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    const dpr = window.devicePixelRatio || 1;
    const width = this.canvas.width;
    const height = this.canvas.height;
    const defaultCenter = [width * 0.5, height * 0.5];

    // 1. Ambient Slow Distortion
    if (this.isAmbientRunning && this.progAmbient) {
      gl.useProgram(this.progAmbient);
      this.bindQuad(this.progAmbient);

      const elapsed = (timestamp - this.ambientStartTime) / 1000.0;
      gl.uniform1f(this.uAmbientTime, elapsed);
      gl.uniform1f(this.uAmbientOffset, this.ambientTimeOffset);
      gl.uniform2f(this.uAmbientRes, width, height);
      gl.uniform2f(this.uAmbientCenter, defaultCenter[0], defaultCenter[1]);

      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }

    // 2. Ripple Distortion
    if (this.ripple && this.progRipple) {
      const elapsed = (timestamp - this.ripple.startTime) / 1000.0;
      if (elapsed >= this.ripple.duration) {
        const cb = this.ripple.onComplete;
        this.ripple = null;
        if (cb) cb();
      } else {
        gl.useProgram(this.progRipple);
        this.bindQuad(this.progRipple);

        const rect = this.canvas.getBoundingClientRect();
        const rx = (this.ripple.originX - rect.left) * dpr;
        // Invert Y for WebGL coordinates
        const ry = (rect.height - (this.ripple.originY - rect.top)) * dpr;

        gl.uniform1f(this.uRippleTime, elapsed);
        gl.uniform2f(this.uRippleRes, width, height);
        gl.uniform2f(this.uRippleCenter, rx, ry);
        gl.uniform4f(
          this.uRippleColor,
          this.ripple.color[0],
          this.ripple.color[1],
          this.ripple.color[2],
          this.ripple.color[3]
        );

        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }
    }

    // 3. High-Energy Burst Distortion
    if (this.burst && this.progBurst) {
      const elapsed = (timestamp - this.burst.startTime) / 1000.0;
      if (elapsed >= this.burst.duration) {
        const cb = this.burst.onComplete;
        this.burst = null;
        if (cb) cb();
      } else {
        gl.useProgram(this.progBurst);
        this.bindQuad(this.progBurst);

        gl.uniform1f(this.uBurstTime, elapsed);
        gl.uniform1f(this.uBurstOffset, this.burst.timeOffset);
        gl.uniform2f(this.uBurstRes, width, height);
        gl.uniform2f(this.uBurstCenter, defaultCenter[0], defaultCenter[1]);

        gl.drawArrays(gl.TRIANGLES, 0, 6);
      }
    }
  }

  private bindQuad(program: WebGLProgram): void {
    const gl = this.gl;
    if (!gl || !this.quadBuffer) return;
    const posAttr = gl.getAttribLocation(program, 'a_position');
    if (posAttr < 0) return;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quadBuffer);
    gl.enableVertexAttribArray(posAttr);
    gl.vertexAttribPointer(posAttr, 2, gl.FLOAT, false, 0, 0);
  }

  // ---------- Procedural Canvas 2D Fallback ----------

  private renderFallback2D(timestamp: number): void {
    const ctx = this.ctx2d;
    const canvas = this.canvas;
    if (!ctx || !canvas) return;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const dpr = window.devicePixelRatio || 1;
    const cx = canvas.width * 0.5;
    const cy = canvas.height * 0.5;

    // 1. Ambient Fallback
    if (this.isAmbientRunning) {
      const elapsed = (timestamp - this.ambientStartTime) / 1000.0;
      const grad = ctx.createRadialGradient(cx, cy, 10 * dpr, cx, cy, Math.min(cx, cy));
      grad.addColorStop(0, 'rgba(10, 6, 18, 0.85)');
      grad.addColorStop(0.5, 'rgba(40, 18, 70, 0.45)');
      grad.addColorStop(1, 'rgba(12, 8, 20, 0.7)');

      ctx.save();
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      // Swirling arcs
      ctx.translate(cx, cy);
      ctx.rotate(elapsed * 0.4);
      ctx.strokeStyle = 'rgba(139, 92, 246, 0.25)';
      ctx.lineWidth = 2 * dpr;
      ctx.beginPath();
      ctx.ellipse(0, 0, cx * 0.6, cy * 0.35, Math.PI / 4, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }

    // 2. Ripple Fallback
    if (this.ripple) {
      const elapsed = (timestamp - this.ripple.startTime) / 1000.0;
      if (elapsed >= this.ripple.duration) {
        const cb = this.ripple.onComplete;
        this.ripple = null;
        if (cb) cb();
      } else {
        const progress = elapsed / this.ripple.duration;
        const rect = canvas.getBoundingClientRect();
        const rx = (this.ripple.originX - rect.left) * dpr;
        const ry = (this.ripple.originY - rect.top) * dpr;
        const maxR = Math.max(canvas.width, canvas.height) * 0.7;

        ctx.save();
        ctx.strokeStyle = `rgba(168, 85, 247, ${0.9 * (1.0 - progress)})`;
        ctx.lineWidth = (3 + (1.0 - progress) * 4) * dpr;
        ctx.beginPath();
        ctx.arc(rx, ry, progress * maxR, 0, Math.PI * 2);
        ctx.stroke();

        ctx.strokeStyle = `rgba(192, 132, 252, ${0.6 * (1.0 - progress)})`;
        ctx.lineWidth = 2 * dpr;
        ctx.beginPath();
        ctx.arc(rx, ry, Math.max(0, progress * maxR - 20 * dpr), 0, Math.PI * 2);
        ctx.stroke();
        ctx.restore();
      }
    }

    // 3. Burst Fallback
    if (this.burst) {
      const elapsed = (timestamp - this.burst.startTime) / 1000.0;
      if (elapsed >= this.burst.duration) {
        const cb = this.burst.onComplete;
        this.burst = null;
        if (cb) cb();
      } else {
        const progress = elapsed / this.burst.duration;
        const fade = 1.0 - progress;
        const radius = progress * Math.max(canvas.width, canvas.height) * 0.9;

        ctx.save();
        const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(1, radius));
        grad.addColorStop(0, `rgba(255, 200, 150, ${0.8 * fade})`);
        grad.addColorStop(0.4, `rgba(255, 90, 40, ${0.6 * fade})`);
        grad.addColorStop(0.8, `rgba(168, 85, 247, ${0.4 * fade})`);
        grad.addColorStop(1, 'rgba(0, 0, 0, 0)');

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(cx, cy, radius, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      }
    }
  }
}

export const voidShader = new VoidShaderController();
