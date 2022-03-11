use uuid::Uuid;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use lyon::geom::math::{point, Point, Vector, vector, Angle, Transform};
use lyon::path::{Path};

use lyon::tessellation::{
    geometry_builder,
    StrokeTessellator, StrokeOptions, LineCap, LineJoin,
    FillTessellator, FillOptions, VertexBuffers,
};

use crate::error::convert_tessellation_error;
use crate::webgl_wrapper::WebGlWrapper;
// pub struct ArrowSettings {
//     length : ArrowLength,
//     width : ArrowDimension,
//     inset : ArrowDimension,
//     scale_length : f32,
//     scale_width : f32,
//     arc : Angle,
//     reverse : bool,
//     harpoon : bool,
//     color : (),
//     fill_color : (),
//     line_cap : (),
//     line_join : (),
//     line_width : ArrowDimension,

// }

// impl ArrowSettings {
//     fn set_length(dim : f32, line_width_factor : f32){

//     }

//     fn set_width(dim : f32, line_width_factor : f32){

//     }
// }

// pub struct ArrowLength {
//     dimension : f32,
//     line_width_factor : f32
// }

// pub struct ArrowDimension {
//     dimension : f32,
//     line_width_factor : f32,
//     length_factor : f32,
// }

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub struct ArrowId(Uuid);

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Arrow {
    pub(crate) tip_end : f32,
    pub(crate) back_end : f32,
    pub(crate) visual_tip_end : f32,
    pub(crate) visual_back_end : f32,
    pub(crate) line_end : f32,
    pub(crate) path : Rc<Path>,
    pub(crate) stroke : Option<StrokeOptions>,
    pub(crate) fill : Option<FillOptions>,
    pub(crate) uuid : ArrowId,
}

impl Arrow {
    pub fn tesselate_into_buffers(&self, buffers : &mut VertexBuffers<Point, u16>) -> Result<(), JsValue> {
        let mut vertex_builder = geometry_builder::simple_builder(buffers);
        let mut fill = FillTessellator::new();
        let mut stroke = StrokeTessellator::new();

        if let Some(fill_options) = &self.fill {
            fill.tessellate(self.path.iter(), fill_options, &mut vertex_builder).map_err(convert_tessellation_error)?;
        }
        if let Some(stroke_options) = &self.stroke {
            stroke.tessellate(self.path.iter(), stroke_options, &mut vertex_builder).map_err(convert_tessellation_error)?;
        }
        Ok(())
    }
}

#[wasm_bindgen]
impl Arrow {
    // length = +1.6pt 2.2, // 1.6pt + 2.2 * line_width
    // width' = +0pt 2.096774, // 2.096774 * length
    // line width = 0pt 1 1, // line width is normal line width
    // round

    // \pgfpathmoveto
    // {\pgfqpoint{-\pgfutil@tempdima}{.5\pgfutil@tempdimb}}
    // \pgfpathcurveto
    // {\pgfqpoint{-0.81731\pgfutil@tempdima}{.2\pgfutil@tempdimb}}
    // {\pgfqpoint{-0.41019\pgfutil@tempdima}{0.05833333\pgfutil@tempdimb}}
    // {\pgfpointorigin}
    // \pgfpathcurveto
    // {\pgfqpoint{-0.41019\pgfutil@tempdima}{-0.05833333\pgfutil@tempdimb}}
    // {\pgfqpoint{-0.81731\pgfutil@tempdima}{-.2\pgfutil@tempdimb}}
    // {\pgfqpoint{-\pgfutil@tempdima}{-.5\pgfutil@tempdimb}}


    // \ifpgfarrowroundjoin%
    // \else%
    //   \pgfmathdivide@{\pgf@sys@tonumber\pgfutil@tempdima}{\pgf@sys@tonumber\pgfutil@tempdimb}%
    //   \let\pgf@temp@quot\pgfmathresult%
    //   \pgf@x\pgfmathresult pt%
    //   \pgf@x\pgfmathresult\pgf@x%
    //   \pgf@x49.44662\pgf@x%
    //   \advance\pgf@x by1pt%  \pgfarrowlinewidth^2 + (0.41019/0.0583333 \pgftempdim@a / \pgfutil@tempdimb) \pgfarrowlinewidth^2
    //   \pgfmathsqrt@{\pgf@sys@tonumber\pgf@x}%
    //   \pgf@xc\pgfmathresult\pgfarrowlinewidth% xc is front miter
    //   \pgf@xc.5\pgf@xc
    //   \pgf@xa\pgf@temp@quot\pgfarrowlinewidth% xa is extra harpoon miter
    //   \pgf@xa3.51591\pgf@xa% xa is extra harpoon miter
    // \fi%
    // \pgfarrowssettipend{\ifpgfarrowroundjoin.5\pgfarrowlinewidth\else\pgf@xc\ifpgfarrowharpoon\advance\pgf@x by\pgf@xa\fi\fi}

    pub fn normal_arrow(line_width : f32, round_join : bool, round_cap : bool, harpoon : bool, reversed : bool) -> Self {
        let length = line_width * 2.2 + WebGlWrapper::point_to_pixels(1.6);
        let width = 2.096774 * length;
        let length = length - line_width;
        let width = width - line_width;


        let mut tip_end = if round_join {
                line_width / 2.0
            } else {
                let miter = ((length / width) * (length / width) * 49.44662 + 1.0).sqrt() * line_width;
                if harpoon {
                    let extra_harpoon_miter = 3.51591 * (length / width) * line_width;
                    miter + extra_harpoon_miter
                } else {
                    miter
                }
            };
        let mut visual_tip_end = tip_end;

        let mut visual_back_end = - line_width / 2.0;
        let mut back_end = - length - line_width / 2.0;

    //     \ifpgfarrowreversed%
    //     \ifpgfarrowharpoon%
    //       \pgfarrowssetlineend{.5\pgfarrowlinewidth}%
    //     \else%
    //       \pgfarrowssetlineend{-.5\pgfarrowlinewidth}%
    //     \fi%
    //   \else%
    //     \pgfarrowssetlineend{-.5\pgfarrowlinewidth}%
    //   \fi%
        let line_end = if reversed {
            if harpoon {
                line_width/2.0
            } else {
                - line_width/2.0
            }
        } else {
            - line_width/2.0
        };


        let mut path_builder = Path::builder();
        path_builder.move_to(point(-length, width/2.0));

        path_builder.cubic_bezier_to(
            point(-0.81731 * length, 0.2 * width),
            point(-0.41019 * length, 0.05833333 * width),
            point(0.0, 0.0)
        );
        if harpoon {
            path_builder.line_to(point(if reversed { 0.5 } else { -1.0 } * line_width, 0.0));
        } else {
            path_builder.cubic_bezier_to(
                point(-0.41019 * length, -0.05833333 * width),
                point(-0.81731 * length, -0.2 * width),
                point(-length, -width/2.0)
            );
        }
        let path = Rc::new(path_builder.build());

        // let path = Rc::new(if reversed {
        //     path.transformed(&Transform::scale(-1.0, 1.0))
        // } else {
        //     path
        // });

        let stroke_options = StrokeOptions::DEFAULT
            .with_line_join(
                if round_join { LineJoin::Round } else { LineJoin::MiterClip }
            ).with_line_cap(
                if round_cap { LineCap::Round } else { LineCap::Butt }
            ).with_line_width(
                line_width
            );

        if reversed {
            std::mem::swap(&mut visual_back_end, &mut visual_tip_end);
            std::mem::swap(&mut back_end, &mut tip_end);
        }

        Self {
            tip_end,
            back_end,
            visual_tip_end,
            visual_back_end,
            line_end,
            path,
            stroke : Some(stroke_options),
            fill : None,
            uuid : ArrowId(Uuid::new_v4())
        }
    }

    // defaults = {
    //     length = +0.75pt 1.25,
    //     width'  = +0pt 4 -1,
    //     line width = +0pt 1 1,
    //   },

    pub fn hook_arrow(line_width : f32, angle : f32, round_join : bool, round_cap : bool, harpoon : bool, reversed : bool) -> Self {
    //     % Adjust width and length: Take line thickness into account:
    //     \advance\pgfarrowlength by-.5\pgfarrowlinewidth
    //     \advance\pgfarrowwidth by-\pgfarrowlinewidth
        let length = line_width * 1.25 + WebGlWrapper::point_to_pixels(0.75);
        let width = 4.0 * length - line_width;
        let length = length - line_width / 2.0;
        let width = width - line_width;
        let angle = Angle::degrees(angle);



    //     \ifpgfarrowreversed
    //       \ifpgfarrowroundjoin
    //         \pgfarrowssetbackend{-.5\pgfarrowlinewidth}
    //       \fi
    //     \fi


        let (sin_angle, _) = angle.sin_cos();
        let tip_end = line_width / 2.0 + length * if reversed {
            if angle > Angle::frac_pi_2() * 3.0 { 1.0 } else if angle > Angle::pi() { -sin_angle } else { 0.0 }
        } else {
            if angle < Angle::frac_pi_2() { sin_angle } else { 1.0 }
        };

        //     % There are four different intervals for the values of
    //     % \pgfarrowsarc that give rise to four different settings of tip
    //     % ends and so on:
    //     %
    //     % Case 1: 0 <= Angle < 90
    //     %

        let back_end = - line_width / 2.0 + if angle < Angle::frac_pi_2() {
            if reversed && round_join {
                0.0
            } else {
                line_width / 2.0
            }
        } else if angle < Angle::pi() {
            if round_cap {
                0.0
            } else {
                line_width / 2.0
            }
        } else if angle < Angle::frac_pi_2() * 3.0 {
            sin_angle * length
        } else {
            - length
        };

    //     \ifdim\pgfarrowarc pt<90pt%
    //     \else\ifdim\pgfarrowarc pt<180pt%
    //       \ifpgfarrowroundcap\pgfarrowssetbackend{-.5\pgfarrowlinewidth}\fi%
    //     \else\ifdim\pgfarrowarc pt<270pt%
    //         % Back end is given by sin(pgfarrowarc)*length
    //         \pgfmathsin@{\pgfarrowarc}
    //         \pgfarrowssetbackend{\pgfmathresult\pgfarrowlength\advance\pgf@x by-.5\pgfarrowlinewidth}%
    //     \else%
    //       \pgfarrowssetbackend{-\pgfarrowlength\advance\pgf@x by-.5\pgfarrowlinewidth}%
    //     \fi\fi\fi%



    //     \ifpgfarrowreversed
    //       \pgfarrowssetlineend{.5\pgfarrowlinewidth}
    //     \else%
    //       \ifpgfarrowharpoon
    //         \pgfarrowssetlineend{0pt}
    //       \else
    //         \pgfarrowssetlineend{.25\pgfarrowlinewidth}
    //       \fi
    //     \fi

        let line_end = if reversed {
            line_width / 2.0
        } else if harpoon {
            0.0
        } else {
            line_width / 4.0
        };

        // \pgfsetdash{}{+0pt}
        // \ifpgfarrowroundjoin\pgfsetroundjoin\else\pgfsetmiterjoin\fi
        // \ifpgfarrowroundcap\pgfsetroundcap\else\pgfsetbuttcap\fi
        // \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
        let stroke_options = StrokeOptions::DEFAULT
            .with_line_join(
                if round_join { LineJoin::Round } else { LineJoin::MiterClip }
            ).with_line_cap(
                if round_cap { LineCap::Round } else { LineCap::Butt }
            ).with_line_width(
                line_width
            );

        // {%
        //   \pgftransformxscale{+\pgfarrowlength}
        //   \pgftransformyscale{+.25\pgfarrowwidth}
        //   \pgfpathmoveto{\pgfpointpolar{+\pgfarrowarc}{+1pt}\advance\pgf@y by1pt}
        //   \pgfpatharc{\pgfarrowarc}{+-90}{+1pt}
        //   \ifpgfarrowharpoon
        //   \else
        //     \pgfpatharc{+90}{+-\pgfarrowarc}{+1pt}
        //   \fi
        // }
        // \ifpgfarrowharpoon\ifpgfarrowreversed
        // \pgfpathlineto{\pgfqpoint{\pgflinewidth}{0pt}}
        // \fi\fi
        // \pgfusepathqstroke


        let mut path_builder = Path::builder();
        path_builder.move_to(point(0.0, 1.0) + Vector::from_angle_and_length(angle-Angle::frac_pi_2(), 1.0));
        path_builder.arc(point(0.0, 1.0), vector(1.0, 1.0), -angle, Angle::zero());
        if !harpoon {
            path_builder.arc(point(0.0, -1.0), vector(1.0, 1.0), -angle, Angle::zero());
        }
        if harpoon && reversed {
            path_builder.line_to(point(line_width / length, 0.0));
        }
        let path = Rc::new(path_builder.build().transformed(
            &Transform::scale(length * if reversed { -1.0 } else { 1.0 }, width/4.0))
        );


        Self {
            tip_end,
            back_end,
            visual_tip_end : tip_end,
            visual_back_end : back_end,
            line_end,
            path,
            stroke : Some(stroke_options),
            fill : None,
            uuid : ArrowId(Uuid::new_v4())
        }
    }



    //     % Adjust arc:
    //     \pgf@x\pgfarrowarc pt%
    //     \advance\pgf@x by-90pt%
    //     \edef\pgfarrowarc{\pgf@sys@tonumber\pgf@x}%
    //     % The following are needed in the code:
    //     \pgfarrowssavethe\pgfarrowlinewidth
    //     \pgfarrowssavethe\pgfarrowlength
    //     \pgfarrowssavethe\pgfarrowwidth
    //     \pgfarrowssave\pgfarrowarc
    //   },
    //   drawing code = {
    //     \pgfsetdash{}{+0pt}
    //     \ifpgfarrowroundjoin\pgfsetroundjoin\else\pgfsetmiterjoin\fi
    //     \ifpgfarrowroundcap\pgfsetroundcap\else\pgfsetbuttcap\fi
    //     \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
    //     {%
    //       \pgftransformxscale{+\pgfarrowlength}
    //       \pgftransformyscale{+.25\pgfarrowwidth}
    //       \pgfpathmoveto{\pgfpointpolar{+\pgfarrowarc}{+1pt}\advance\pgf@y by1pt}
    //       \pgfpatharc{\pgfarrowarc}{+-90}{+1pt}
    //       \ifpgfarrowharpoon
    //       \else
    //         \pgfpatharc{+90}{+-\pgfarrowarc}{+1pt}
    //       \fi
    //     }
    //     \ifpgfarrowharpoon\ifpgfarrowreversed
    //     \pgfpathlineto{\pgfqpoint{\pgflinewidth}{0pt}}
    //     \fi\fi
    //     \pgfusepathqstroke
    //   },

    pub fn test_arrow() -> Self {
        let length = 30.0;
        let width = 2.096774 * length;
        let mut path_builder = Path::builder();
        path_builder.move_to(point(-length, width/2.0));
        path_builder.line_to(point(0.0, 0.0));
        path_builder.line_to(point(-length, -width/2.0));
        path_builder.line_to(point(-length/2.0, 0.0));
        path_builder.close();
        let path = Rc::new(path_builder.build());
        let tip_end = 0.0;
        let visual_tip_end = 0.0;
        let back_end = -length;
        let visual_back_end = - length/2.0;
        let line_end = -length/3.0;
        Self {
            tip_end,
            back_end,
            visual_tip_end,
            visual_back_end,
            line_end,
            path,
            // fill : Some(FillOptions::DEFAULT),
            fill : None,
            stroke : Some(StrokeOptions::DEFAULT),
            // stroke : None,
            uuid : ArrowId(Uuid::new_v4())
        }
    }
}
