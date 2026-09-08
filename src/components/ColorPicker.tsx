import { COLOR_PALETTE, type PrimaryColorName } from '@/lib/usePrimaryColor';

interface ColorPickerProps {
  colorName: PrimaryColorName;
  onColorChange: (name: PrimaryColorName) => void;
}

export function ColorPicker({ colorName, onColorChange }: ColorPickerProps) {
  const selectedColor = COLOR_PALETTE.find(color => color.name === colorName);

  return (
    <fieldset className="flex flex-col gap-3">
      <legend className="mb-3 text-sm">
        Accent color <span className="text-muted-foreground">· {selectedColor?.label}</span>
      </legend>
      <div className="grid w-fit grid-cols-6 gap-2 sm:grid-cols-9">
        {COLOR_PALETTE.map(color => (
          <button
            key={color.name}
            type="button"
            onClick={() => onColorChange(color.name)}
            className={`h-8 w-8 rounded-sm border transition-transform hover:scale-110 focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-4 ${
              colorName === color.name
                ? 'ring-2 ring-foreground ring-offset-2 ring-offset-background'
                : ''
            }`}
            style={{ background: color.value ?? 'linear-gradient(135deg, #fff 45%, #000 55%)' }}
            title={color.label}
            aria-label={color.label}
            aria-pressed={colorName === color.name}
          />
        ))}
      </div>
    </fieldset>
  );
}
