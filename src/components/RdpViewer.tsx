import { useRef, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Props {
  sessionId: number;
  width: number;
  height: number;
  onClose: () => void;
}

export function RdpViewer({ sessionId, width, height, onClose }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rafRef = useRef<number>(0);

  // Poll framebuffer ~20fps
  useEffect(() => {
    let running = true;
    const poll = async () => {
      if (!running) return;
      try {
        const pixels = await invoke<number[]>('getFrameBuffer', { sessionId });
        const canvas = canvasRef.current;
        if (canvas && pixels.length > 0) {
          const ctx = canvas.getContext('2d')!;
          const imgData = new ImageData(
            new Uint8ClampedArray(new Uint32Array(pixels).buffer),
            width, height
          );
          ctx.putImageData(imgData, 0, 0);
        }
      } catch { /* session closed */ }
      if (running) rafRef.current = requestAnimationFrame(() => setTimeout(poll, 50));
    };
    poll();
    return () => { running = false; cancelAnimationFrame(rafRef.current); };
  }, [sessionId, width, height]);

  const send = useCallback((event: Record<string, unknown>) => {
    invoke('sendRdpInput', { sessionId, event }).catch(() => {});
  }, [sessionId]);

  const handleMouseMove = (e: React.MouseEvent) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const scaleX = width / rect.width;
    const scaleY = height / rect.height;
    send({ event_type: 'mouse_move', x: Math.round((e.clientX - rect.left) * scaleX), y: Math.round((e.clientY - rect.top) * scaleY), button: 0, key_code: 0, is_down: false });
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const scaleX = width / rect.width;
    const scaleY = height / rect.height;
    send({ event_type: 'mouse_down', x: Math.round((e.clientX - rect.left) * scaleX), y: Math.round((e.clientY - rect.top) * scaleY), button: e.button + 1, key_code: 0, is_down: true });
  };

  const handleMouseUp = (e: React.MouseEvent) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    const scaleX = width / rect.width;
    const scaleY = height / rect.height;
    send({ event_type: 'mouse_up', x: Math.round((e.clientX - rect.left) * scaleX), y: Math.round((e.clientY - rect.top) * scaleY), button: e.button + 1, key_code: 0, is_down: false });
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    send({ event_type: 'key_down', x: 0, y: 0, button: 0, key_code: e.keyCode, is_down: true });
  };

  const handleKeyUp = (e: React.KeyboardEvent) => {
    e.preventDefault();
    send({ event_type: 'key_up', x: 0, y: 0, button: 0, key_code: e.keyCode, is_down: false });
  };

  return (
    <div className="rdp-viewer">
      <div className="rdp-viewer-bar">
        <span>RDP Session #{sessionId}</span>
        <button className="btn-icon" onClick={onClose} title="Disconnect">✕</button>
      </div>
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        onMouseMove={handleMouseMove}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        tabIndex={0}
        className="rdp-canvas"
      />
    </div>
  );
}
