const HandyTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 940 140"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* textLength pins glyph advances so the wordmark ends at x=845 on every
          platform; without it, fallback monospace fonts (~0.6em advance vs
          0.55em) push the text under the cursor rect at x=858. IBM Plex Mono
          is self-hosted at 500/600 only — don't ask for other weights. */}
      <text
        x="0"
        y="102"
        fontFamily="'IBM Plex Mono', Consolas, 'Cascadia Mono', ui-monospace, monospace"
        fontSize="96"
      >
        {/* eslint-disable i18next/no-literal-string -- brand name is not translated */}
        <tspan
          fontWeight="600"
          className="logo-primary"
          textLength="264"
          lengthAdjust="spacingAndGlyphs"
        >
          klava
        </tspan>
        <tspan
          fontWeight="500"
          fill="var(--color-mid-gray)"
          x="264"
          textLength="581"
          lengthAdjust="spacingAndGlyphs"
        >
          -nevinovata
        </tspan>
        {/* eslint-enable i18next/no-literal-string */}
      </text>
      <rect
        x="858"
        y="26"
        width="24"
        height="88"
        fill="var(--color-logo-primary)"
      />
    </svg>
  );
};

export default HandyTextLogo;
