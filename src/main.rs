use std::io::Write;

use anyhow::{ensure, Result};

unsafe fn decompress(buffer: &Vec<u8>) -> Result<mozjpeg_sys::jpeg_decompress_struct> {
  let mut err: mozjpeg_sys::jpeg_error_mgr = std::mem::zeroed();
  let mut cinfo: mozjpeg_sys::jpeg_decompress_struct = std::mem::zeroed();
  cinfo.common.err = mozjpeg_sys::jpeg_std_error(&mut err);
  mozjpeg_sys::jpeg_create_decompress(&mut cinfo);

  mozjpeg_sys::jpeg_mem_src(&mut cinfo, buffer.as_ptr(), buffer.len().try_into()?);

  Ok(cinfo)
}

unsafe fn dump_blocks(
  cinfo: &mut mozjpeg_sys::jpeg_decompress_struct,
  coefs_ptr: *mut *mut mozjpeg_sys::jvirt_barray_control,
  comp: Option<String>,
) -> Result<()> {
  let mut buffer;
  let mut stdout = std::io::stdout().lock();

  let comp_range = match comp {
    Some(comp) => {
      let comp: u32 = comp.parse()?;
      ensure!(comp < cinfo.num_components as u32, "Invalid component #{comp}");
      let comp = comp as isize;
      comp..comp + 1
    }
    None => 0..cinfo.num_components as isize,
  };

  for comp in comp_range {
    let comp_info = cinfo.comp_info.offset(comp);
    for blk_y in (0..(*comp_info).height_in_blocks).step_by((*cinfo.comp_info).v_samp_factor as usize) {
      buffer = (*cinfo.common.mem).access_virt_barray.unwrap()(
        &mut cinfo.common,
        *coefs_ptr.offset(comp),
        blk_y,
        (*comp_info).v_samp_factor as u32,
        1,
      );
      for offset_y in 0..(*comp_info).v_samp_factor {
        let block = *buffer.offset(offset_y as isize);
        for blk_x in 0..(*comp_info).width_in_blocks {
          for coef in (*block.offset(blk_x as isize)).iter_mut() {
            stdout.write_all(&coef.to_le_bytes())?;
          }
        }
      }
      stdout.flush()?;
    }
  }
  Ok(())
}

fn main() -> Result<()> {
  let image_buf = std::fs::read(std::env::args().nth(1).unwrap())?;
  unsafe {
    let mut cinfo = decompress(&image_buf)?;
    mozjpeg_sys::jpeg_read_header(&mut cinfo, true as mozjpeg_sys::boolean);

    let coefs_ptr = mozjpeg_sys::jpeg_read_coefficients(&mut cinfo);
    dump_blocks(&mut cinfo, coefs_ptr, std::env::args().nth(2))?;
  };
  Ok(())
}
