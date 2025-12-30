interface DateHeaderProps {
  label: string;
  photoCount: number;
}

export function DateHeader({ label, photoCount }: DateHeaderProps) {
  return (
    <div className="px-6 py-3 flex items-baseline gap-2">
      <h2 className="text-sm font-semibold text-gray-700 dark:text-gray-300">{label}</h2>
      <span className="text-xs text-gray-400 dark:text-gray-500">
        {photoCount} {photoCount === 1 ? 'photo' : 'photos'}
      </span>
    </div>
  );
}
