"use client";
import React, { useEffect, useRef, useState } from "react";
import {
  useScroll,
  useTransform,
  useReducedMotion,
  motion,
  MotionValue,
} from "motion/react";

export const ContainerScroll = ({
  titleComponent,
  children,
}: {
  titleComponent: string | React.ReactNode;
  children: React.ReactNode;
}) => {
  const cardWrapRef = useRef<HTMLDivElement>(null);
  const { scrollY } = useScroll();

  const shouldReduceMotion = useReducedMotion();
  const [isMobile, setIsMobile] = useState(false);
  const [centerScrollY, setCenterScrollY] = useState<number>(480);

  useEffect(() => {
    const measure = () => {
      const mobile = window.innerWidth <= 768;
      setIsMobile(mobile);

      const el = cardWrapRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const docTop = window.scrollY + rect.top;
      const targetCenter = docTop + rect.height / 2 - window.innerHeight / 2;
      setCenterScrollY(Math.max(120, Math.round(targetCenter)));
    };

    measure();
    window.addEventListener("resize", measure, { passive: true });

    let ro: ResizeObserver | undefined;
    if (typeof ResizeObserver !== "undefined" && cardWrapRef.current) {
      ro = new ResizeObserver(measure);
      ro.observe(cardWrapRef.current);
    }

    return () => {
      window.removeEventListener("resize", measure);
      ro?.disconnect();
    };
  }, []);

  const scaleRange: [number, number] = isMobile ? [0.92, 1] : [0.94, 1];

  const rotate = useTransform(
    scrollY,
    [0, centerScrollY],
    shouldReduceMotion ? [0, 0] : [24, 0]
  );
  const scale = useTransform(
    scrollY,
    [0, centerScrollY],
    shouldReduceMotion ? [1, 1] : scaleRange
  );
  const cardY = useTransform(
    scrollY,
    [0, centerScrollY],
    shouldReduceMotion ? [0, 0] : [20, 0]
  );
  const headerY = useTransform(
    scrollY,
    [0, centerScrollY],
    shouldReduceMotion ? [0, 0] : [0, -28]
  );

  return (
    <div className="flex items-center justify-center relative px-3 md:px-10 pt-2 pb-6 md:pb-10">
      <div className="pt-2 pb-4 md:pt-4 md:pb-8 w-full relative">
        <Header translate={headerY} titleComponent={titleComponent} />
        <div
          ref={cardWrapRef}
          data-scroll-perspective="true"
          className="mt-20 md:mt-32"
          style={{ perspective: "1000px" }}
        >
          <Card rotate={rotate} scale={scale} translateY={cardY}>
            {children}
          </Card>
        </div>
      </div>
    </div>
  );
};

export const Header = ({
  translate,
  titleComponent,
}: {
  translate: MotionValue<number>;
  titleComponent: string | React.ReactNode;
}) => {
  return (
    <motion.div
      style={{
        translateY: translate,
      }}
      className="max-w-5xl mx-auto text-center"
    >
      {titleComponent}
    </motion.div>
  );
};

export const Card = ({
  rotate,
  scale,
  translateY,
  children,
}: {
  rotate: MotionValue<number>;
  scale: MotionValue<number>;
  translateY: MotionValue<number>;
  children: React.ReactNode;
}) => {
  return (
    <motion.div
      data-scroll-card="true"
      style={{
        rotateX: rotate,
        scale,
        y: translateY,
        transformOrigin: "center center",
        willChange: "transform",
        boxShadow:
          "0 0 #0000004d, 0 12px 28px #0000004a, 0 37px 45px #00000042, 0 70px 60px #00000026",
      }}
      className="max-w-[1040px] mx-auto w-full border-2 border-[#2a3126] p-2 md:p-3 bg-[#141814] rounded-[24px] shadow-2xl"
    >
      <div className="w-full overflow-hidden rounded-[18px] bg-[#0d0f0d]">
        {children}
      </div>
    </motion.div>
  );
};
