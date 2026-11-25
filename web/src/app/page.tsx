import React from "react";
import { BentoShowcase } from "../components/BentoShowcase";
import { FaqSection } from "../components/FaqSection";
import GradientASCII from "../components/GradientASCII";
import { HowItWorksSection } from "../components/HowItWorksSection";
import { ArrowDownRightIcon, SparklesIcon } from "../components/Icons";
import { InstallSection } from "../components/InstallSection";
import { Navbar } from "../components/Navbar";
import { StatementReveal } from "../components/StatementReveal";
import { TestimonialsSection } from "../components/TestimonialsSection";
import { ContainerScroll } from "../components/ui/container-scroll-animation";
import { WorkbenchDemo } from "../components/WorkbenchDemo";
import { MARQUEE_ITEMS } from "../data/showcase-data";

export default function ShowcasePage() {
  const repeatedMarquee = [...MARQUEE_ITEMS, ...MARQUEE_ITEMS];

  return (
    <div id="top" className="page-frame">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      <Navbar />

      <main id="main-content">
        <div className="hero-outer">
          <section className="hero-card" aria-labelledby="hero-heading">
            <div className="hero-ascii-layer" aria-hidden="true">
              <GradientASCII
                colors={["#8cb953ff", "#6ca92c", "#a8d648", "#eef8d6"]}
                shape="horizon"
                cellSize={18}
                density={0.78}
                glyphOpacity={0.42}
                gradientOpacity={0.58}
                glow={0.18}
                softness={0.62}
                speed={1}
                lightMode
                intro
                clickEffect="wave"
                mouseInteraction
              />
            </div>

            <div className="relative z-[2]">
              <ContainerScroll
                titleComponent={
                  <div className="hero-inner">
                    <div className="hero-badge">
                      <span>Now Available · v0.1.0</span>
                      <SparklesIcon className="hero-badge-spark" />
                    </div>

                    <h1 id="hero-heading">
                      Unscan Faster
                      <br />
                      Edit with <span className="hero-serif-accent">Confidence</span>
                    </h1>

                    <p className="hero-lead">
                      The local desktop workbench for teams who turn messy scans and locked PDFs into
                      editable tables and crisp vector PDFs.
                    </p>

                    <div className="hero-cta-wrap">
                      <a href="#download" className="split-cta split-cta-dark">
                        <span className="split-cta-label">Download Free</span>
                        <span className="split-cta-icon">
                          <ArrowDownRightIcon className="icon-xs" />
                        </span>
                      </a>
                    </div>
                  </div>
                }
              >
                <WorkbenchDemo />
              </ContainerScroll>
            </div>

            <div className="marquee-band" aria-label="Core capabilities marquee">
              <div className="marquee-track">
                {repeatedMarquee.map((label, idx) => (
                  <div key={`${label}-${idx}`} className="marquee-item">
                    <span className="marquee-dot" aria-hidden="true" />
                    <span>{label}</span>
                  </div>
                ))}
              </div>
            </div>
          </section>
        </div>

        <StatementReveal />
        <BentoShowcase />
        <HowItWorksSection />
        <InstallSection />
        <FaqSection />
      </main>
    </div>
  );
}
