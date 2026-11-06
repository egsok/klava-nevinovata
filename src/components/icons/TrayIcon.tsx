/** Same microphone + halo geometry as rebrand/generate.mjs. */
type TrayIconState = "idle" | "recording" | "transcribing";

const TrayIcon = ({
  state,
  size,
}: {
  state: TrayIconState;
  size: number | string;
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 64 64"
    className="fill-current"
    xmlns="http://www.w3.org/2000/svg"
  >
    <ellipse
      cx="32.5"
      cy="9.2"
      rx="16.5"
      ry="4.8"
      fill="none"
      stroke="currentColor"
      strokeWidth="5.5"
      transform="rotate(-9 32.5 9.2)"
    />
    {state === "transcribing" ? (
      <g fill="currentColor">
        <circle cx="19" cy="33" r="4.8" />
        <circle cx="32" cy="33" r="4.8" />
        <circle cx="45" cy="33" r="4.8" />
      </g>
    ) : (
      <rect
        x="21.5"
        y="18"
        width="21"
        height="30"
        rx="10.5"
        fill={state === "recording" ? "currentColor" : "none"}
        stroke="currentColor"
        strokeWidth="5.5"
      />
    )}
    <path
      d="M12.5 37.5a19.5 19.5 0 0 0 39 0 M32 57v2.5"
      fill="none"
      stroke="currentColor"
      strokeWidth="5.5"
      strokeLinecap="round"
    />
  </svg>
);

export default TrayIcon;
