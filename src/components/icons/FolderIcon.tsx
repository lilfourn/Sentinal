interface FolderIconProps {
  size?: number;
  className?: string;
}

export function FolderIcon({ size = 18, className }: FolderIconProps) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 375 375"
      className={className}
    >
      <defs>
        <clipPath id="folder-tab">
          <path d="M 8.046875 39.605469 L 367 39.605469 L 367 125 L 8.046875 125 Z" clipRule="nonzero" />
        </clipPath>
        <clipPath id="folder-body">
          <path d="M 8.046875 89.40625 L 366.945312 89.40625 L 366.945312 337.355469 L 8.046875 337.355469 Z" clipRule="nonzero" />
        </clipPath>
      </defs>
      <g clipPath="url(#folder-tab)">
        <path
          fill="#ffae4b"
          d="M 8.046875 124.25 L 8.046875 71.90625 C 8.046875 54.070312 22.503906 39.609375 40.339844 39.609375 L 109.722656 39.609375 C 116.558594 39.609375 123.21875 41.78125 128.746094 45.808594 L 158.953125 67.824219 L 334.648438 67.824219 C 352.484375 67.824219 366.945312 82.285156 366.945312 100.121094 L 366.945312 124.25 L 8.046875 124.25"
          fillOpacity="1"
          fillRule="nonzero"
        />
      </g>
      <g clipPath="url(#folder-body)">
        <path
          fill="#f9943b"
          d="M 334.648438 89.476562 L 40.339844 89.476562 C 22.503906 89.476562 8.046875 103.933594 8.046875 121.773438 L 8.046875 305.195312 C 8.046875 323.03125 22.503906 337.488281 40.339844 337.488281 L 334.648438 337.488281 C 352.484375 337.488281 366.945312 323.03125 366.945312 305.195312 L 366.945312 121.773438 C 366.945312 103.933594 352.484375 89.476562 334.648438 89.476562"
          fillOpacity="1"
          fillRule="nonzero"
        />
      </g>
    </svg>
  );
}
