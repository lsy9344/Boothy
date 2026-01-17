import { Brush, Circle, Droplet, Layers, RectangleHorizontal, TriangleRight } from 'lucide-react';

export enum Mask {
  All = 'all',
  Brush = 'brush',
  Color = 'color',
  Linear = 'linear',
  Luminance = 'luminance',
  Radial = 'radial',
}

export enum SubMaskMode {
  Additive = 'additive',
  Subtractive = 'subtractive',
}

export enum ToolType {
  Brush = 'brush',
  Eraser = 'eraser',
}

export interface MaskType {
  disabled: boolean;
  icon: any;
  id?: string;
  name: string;
  type: Mask;
}

export interface SubMask {
  id: string;
  mode: SubMaskMode;
  parameters?: any;
  type: Mask;
  visible: boolean;
}

export const MASK_ICON_MAP: Record<Mask, any> = {
  [Mask.All]: RectangleHorizontal,
  [Mask.Brush]: Brush,
  [Mask.Color]: Droplet,
  [Mask.Linear]: TriangleRight,
  [Mask.Luminance]: Layers,
  [Mask.Radial]: Circle,
};

export const MASK_PANEL_CREATION_TYPES: Array<MaskType> = [
  {
    disabled: false,
    icon: TriangleRight,
    name: 'Linear',
    type: Mask.Linear,
  },
  {
    disabled: false,
    icon: Circle,
    name: 'Radial',
    type: Mask.Radial,
  },
  {
    disabled: false,
    icon: Layers,
    id: 'others',
    name: 'Others',
    type: null,
  },
];

export const SUB_MASK_COMPONENT_TYPES: Array<MaskType> = [
  {
    disabled: false,
    icon: TriangleRight,
    name: 'Linear',
    type: Mask.Linear,
  },
  {
    disabled: false,
    icon: Circle,
    name: 'Radial',
    type: Mask.Radial,
  },
  {
    disabled: false,
    icon: Layers,
    id: 'others',
    name: 'Others',
    type: null,
  },
];

export const OTHERS_MASK_TYPES: Array<MaskType> = [
  {
    disabled: false,
    icon: Brush,
    name: 'Brush',
    type: Mask.Brush,
  },
  {
    disabled: false,
    icon: RectangleHorizontal,
    name: 'Whole Image',
    type: Mask.All,
  },
];
