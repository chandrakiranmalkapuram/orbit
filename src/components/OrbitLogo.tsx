import React, { useId } from 'react';

interface OrbitLogoProps {
  size?: number;
  className?: string;
  animate?: boolean;
}

export const OrbitLogo: React.FC<OrbitLogoProps> = ({
  size = 48,
  className = '',
  animate = false,
}) => {
  const uid = useId().replace(/:/g, '');
  const saffronId = `orbit-saffron-${uid}`;
  const blueId = `orbit-blue-${uid}`;
  const coreId = `orbit-core-${uid}`;

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 120 120"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={`orbit-logo-svg ${animate ? 'animate-orbit-spin' : ''} ${className}`}
      aria-hidden="true"
    >
      <defs>
        {/* Surya Saffron Gradient */}
        <linearGradient id={saffronId} x1="0%" y1="100%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#FF3D00" />
          <stop offset="40%" stopColor="#FF7A00" />
          <stop offset="75%" stopColor="#FFAA00" />
          <stop offset="100%" stopColor="#FFD000" />
        </linearGradient>

        {/* Ashoka Sapphire Gradient */}
        <linearGradient id={blueId} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#60A5FA" />
          <stop offset="100%" stopColor="#1D4ED8" />
        </linearGradient>

        {/* Surya Radiant Core */}
        <radialGradient id={coreId} cx="38%" cy="35%" r="65%">
          <stop offset="0%" stopColor="#FFFBEB" />
          <stop offset="25%" stopColor="#FDE68A" />
          <stop offset="55%" stopColor="#F59E0B" />
          <stop offset="85%" stopColor="#EA580C" />
          <stop offset="100%" stopColor="#9A3412" />
        </radialGradient>
      </defs>

      {/* 24-spoke Kala Chakra subtle outer ring */}
      <circle
        cx="60"
        cy="60"
        r="32"
        stroke="#1E3A8A"
        strokeWidth="0.75"
        strokeDasharray="1.5 2.5"
        opacity="0.55"
      />

      {/* Counter Orbit: Sapphire Arc */}
      <g transform="rotate(32 60 60)">
        <ellipse
          cx="60"
          cy="60"
          rx="44"
          ry="15"
          stroke={`url(#${blueId})`}
          strokeWidth="1.8"
          strokeDasharray="3.5 2.5"
          opacity="0.8"
        />
        {/* Emerald Node (Tricolor Harmony) */}
        <circle cx="104" cy="60" r="2.2" fill="#10B981" />
      </g>

      {/* Primary Orbit: Saffron Sweep */}
      <g transform="rotate(-26 60 60)">
        <ellipse
          cx="60"
          cy="60"
          rx="48"
          ry="18"
          stroke={`url(#${saffronId})`}
          strokeWidth="3.2"
          strokeLinecap="round"
        />
        {/* Golden Celestial Node */}
        <circle cx="108" cy="60" r="3" fill="#FFD000" />
        <circle cx="12" cy="60" r="1.8" fill="#FF5500" />
      </g>

      {/* Central Surya Body */}
      <circle cx="60" cy="60" r="17" fill={`url(#${coreId})`} />
      {/* Dark Depth Moat */}
      <circle cx="60" cy="60" r="12" fill="#080d1a" />
      <circle cx="60" cy="60" r="12" stroke="#F59E0B" strokeWidth="0.8" opacity="0.65" />
      {/* Inner Radiant Nucleus */}
      <circle cx="60" cy="60" r="6.8" fill={`url(#${coreId})`} />
      <circle cx="60" cy="60" r="2.5" fill="#FFFFFF" opacity="0.9" />
    </svg>
  );
};
