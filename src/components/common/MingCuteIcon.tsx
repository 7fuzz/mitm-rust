import React from 'react';

export type MingCuteIconName =
  | 'http_line'
  | 'history_line'
  | 'shield_line'
  | 'repeat_line'
  | 'repeater'
  | 'send_plane_line'
  | 'transfer_line'
  | 'websocket_line'
  | 'link_line'
  | 'grid_line'
  | 'tool_line'
  | 'settings_line'
  | 'settings_3_line'
  | 'sun_line'
  | 'moon_line'
  | 'search_line'
  | 'filter_line'
  | 'play_line'
  | 'close_circle_line'
  | 'folder_line'
  | 'folder_2_line'
  | 'folder_open_line'
  | 'folder_archive_line'
  | 'file_code_line'
  | 'eye_close_line'
  | 'eye_line'
  | 'copy_line'
  | 'delete_2_line'
  | 'delete_fill'
  | 'storage_line'
  | 'download_line'
  | 'key_line'
  | 'plus_line'
  | 'add_line'
  | 'edit_line'
  | 'arrow_up_line'
  | 'up_line'
  | 'arrow_down_line'
  | 'down_line'
  | 'check_line'
  | 'close_line'
  | 'chevron_right_line'
  | 'chevron_down_line'
  | 'refresh_line'
  | 'terminal_line'
  | 'lock_line'
  | 'unlock_line'
  | 'pause_line'
  | 'external_link_line'
  | 'code_line'
  | 'calculator_line'
  | 'hash_line'
  | 'power_line'
  | 'power'
  | 'zap_line'
  | 'lightning_line'
  | 'radio_line'
  | 'ban_line'
  | 'fast_forward_line'
  | 'earth_line'
  | 'globe_line'
  | 'loading_line'
  | 'alert_line'
  | 'warning_line'
  | 'file_import_line'
  | 'layout_right_line'
  | 'chevron_left_line'
  | 'expand_line'
  | 'swap_line'
  | 'binary_line'
  | 'clipboard_line';

interface MingCuteIconProps {
  name: MingCuteIconName | string;
  size?: number | string;
  className?: string;
  style?: React.CSSProperties;
}

export const MingCuteIcon: React.FC<MingCuteIconProps> = ({
  name,
  size = 18,
  className = '',
  style,
}) => {
  const iconSize = typeof size === 'number' ? `${size}px` : size;

  const renderSvgPath = () => {
    switch (name) {
      case 'power_line':
      case 'power':
        return (
          <path d="M12 3a1 1 0 0 1 1 1v8a1 1 0 1 1-2 0V4a1 1 0 0 1 1-1zm4.78 2.22a1 1 0 0 1 1.41 1.42A8.96 8.96 0 0 1 21 13a9 9 0 1 1-15.19-6.36a1 1 0 0 1 1.41 1.42A7 7 0 1 0 19 13a6.97 6.97 0 0 0-2.22-4.78z" />
        );
      case 'zap_line':
      case 'lightning_line':
        return (
          <path d="M13 2L3 14h9l-1 8l10-12h-8l1-8z" />
        );
      case 'radio_line':
        return (
          <path d="M12 2A10 10 0 1 0 22 12A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8a8 8 0 0 1-8 8zm0-12a4 4 0 1 0 4 4a4 4 0 0 0-4-4zm0 6a2 2 0 1 1 2-2a2 2 0 0 1-2 2z" />
        );
      case 'ban_line':
        return (
          <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm-6.71 4.71l11 11A8 8 0 0 1 5.29 6.71zm13.42 10.58l-11-11A8 8 0 0 1 18.71 17.29z" />
        );
      case 'fast_forward_line':
        return (
          <path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z" />
        );
      case 'http_line':
      case 'history_line':
        return (
          <>
            <path d="M12 2A10 10 0 1022 12A10 10 0 0012 2zm0 18a8 8 0 118-8a8 8 0 01-8 8z" />
            <path d="M12.5 7v5.25l4.5 2.67-.75 1.23L11 13V7z" />
          </>
        );
      case 'shield_line':
        return (
          <path d="M12 2L4 5v6.09c0 5.05 3.41 9.76 8 10.91c4.59-1.15 8-5.86 8-10.91V5l-8-3zm6 9.09c0 4.08-2.73 7.88-6 8.86c-3.27-.98-6-4.78-6-8.86V6.78l6-2.25l6 2.25v4.31zM11 10h2v5h-2zm0-3h2v2h-2z" />
        );
      case 'repeat_line':
      case 'repeater':
        return (
          <path d="M19 8l-4-4v3H5v4h2V9h8v3l4-4zM5 16l4 4v-3h10v-4h-2v2H9v-3l-4 4z" />
        );
      case 'send_plane_line':
        return (
          <path d="M1.923 9.37c-.51-.205-.504-.51.034-.689l19.086-6.362c.529-.176.832.12.684.638l-5.454 19.086c-.15.529-.475.535-.731.008l-4.28-8.798l-6.26-6.26l-3.079 2.377z" />
        );
      case 'websocket_line':
        return (
          <path d="M9 7.539L15 21.539L18.659 13H23V11H17.341L15 16.461L9 2.461L5.341 11H1V13H6.659L9 7.539Z" />
        );
      case 'swap_line':
        return (
          <path d="M16.05 12.05L21 17l-4.95 4.95-1.414-1.414 2.536-2.537L4 18v-2h13.172l-2.536-2.536 1.414-1.414zm-8.1-10l1.414 1.414L6.828 6 20 6v2H6.828l2.536 2.536L7.95 11.95 3 7l4.95-4.95z" />
        );
      case 'binary_line':
        return (
          <path d="M7 5a3 3 0 0 1 3 3v8a3 3 0 0 1-6 0V8a3 3 0 0 1 3-3zm0 2a1 1 0 0 0-1 1v8a1 1 0 0 0 2 0V8a1 1 0 0 0-1-1zm9-2h2v12h2v2h-6v-2h2V7.8l-1.5.9-1-1.7L16 5z" />
        );
      case 'clipboard_line':
        return (
          <path d="M7 4V2h10v2h3.007c.548 0 .993.445.993.993v16.014a.994.994 0 0 1-.993.993H3.993A.994.994 0 0 1 3 21.007V4.993C3 4.445 3.445 4 3.993 4H7zm0 2H5v14h14V6h-2v2H7V6zm2-2v2h6V4H9z" />
        );
      case 'transfer_line':
        return (
          <path d="M7 4V2h2v2h10a1 1 0 011 1v4a1 1 0 01-1 1H4a1 1 0 01-1-1V5a1 1 0 011-1h3zm-2 4h13V6H5v2zm0 12v-2h2v2H5zm4-2h10a1 1 0 011 1v4a1 1 0 01-1 1H4a1 1 0 01-1-1v-4a1 1 0 011-1h5zm-4 4h13v-2H5v2z" />
        );
      case 'link_line':
        return (
          <path d="M13.06 8.11l1.41-1.41a5 5 0 017.07 7.07l-3.53 3.54a5 5 0 01-7.07 0l-1.42 1.41a7 7 0 009.9 0l3.53-3.54a7 7 0 00-9.9-9.9zm-2.12 7.78l-1.41 1.41a5 5 0 01-7.07-7.07l3.53-3.54a5 5 0 017.07 0l1.42-1.41a7 7 0 00-9.9 0L1.15 9.82a7 7 0 009.9 9.9z" />
        );
      case 'grid_line':
        return (
          <path d="M3 3h8v8H3V3zm10 0h8v8h-8V3zM3 13h8v8H3v-8zm10 0h8v8h-8v-8zM5 5v4h4V5H5zm10 0v4h4V5h-4zM5 15v4h4v-4H5zm10 0v4h4v-4h-4z" />
        );
      case 'tool_line':
        return (
          <path d="M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9c-2-2-5-2.4-7.4-1.3L9 6L6 9L1.6 4.7C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4z" />
        );
      case 'settings_line':
      case 'settings_3_line':
        return (
          <path d="M19.14 12.94c.04-.3.06-.61.06-.94c0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 00.12-.61l-1.92-3.32a.488.488 0 00-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 00-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 00-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.5A3.5 3.5 0 018.5 12A3.5 3.5 0 0112 8.5a3.5 3.5 0 013.5 3.5a3.5 3.5 0 01-3.5 3.5z" />
        );
      case 'sun_line':
        return (
          <path d="M12 18C8.686 18 6 15.314 6 12S8.686 6 12 6s6 2.686 6 6-2.686 6-6 6zm0-2a4 4 0 100-8 4 4 0 000 8zM11 1h2v3h-2V1zm0 19h2v3h-2v-3zM3.515 4.929l1.414-1.414L7.05 5.636 5.636 7.05 3.515 4.93zM16.95 18.364l1.414-1.414 2.121 2.121-1.414 1.414-2.121-2.121zM1 11h3v2H1v-2zm19 0h3v2h-3v-2zM4.929 20.485l-1.414-1.414 2.121-2.121 1.414 1.414-2.121 2.121zM18.364 7.05l-1.414-1.414 2.121-2.121 1.414 1.414-2.121 2.121z" />
        );
      case 'moon_line':
        return (
          <path d="M10 7a7 7 0 0012 4.9v.1A10 10 0 119 2a7 7 0 001 5z" />
        );
      case 'search_line':
        return (
          <path d="M18.031 16.617l4.283 4.282-1.415 1.415-4.282-4.283A8.96 8.96 0 0111 20a9 9 0 119-9a8.96 8.96 0 01-1.969 5.617zm-2.006-.742A6.977 6.977 0 0018 11a7 7 0 10-7 7c1.76 0 3.37-.649 4.61-1.725l.415-.4z" />
        );
      case 'filter_line':
        return (
          <path d="M10 18h4v-2h-4v2zM3 6v2h18V6H3zm3 7h12v-2H6v2z" />
        );
      case 'play_line':
        return (
          <path d="M8 5v14l11-7z" />
        );
      case 'close_circle_line':
        return (
          <path d="M12 2a10 10 0 1010 10A10 10 0 0012 2zm0 18a8 8 0 118-8a8 8 0 01-8 8zm3.54-11.54L13.41 12l2.13 2.12-1.42 1.42L12 13.41l-2.12 2.13-1.42-1.42L10.59 12 8.46 9.88l1.42-1.42L12 10.59l2.12-2.13z" />
        );
      case 'folder_line':
      case 'folder_2_line':
      case 'folder_open_line':
      case 'folder_archive_line':
        return (
          <path d="M12.414 5H21a1 1 0 011 1v14a1 1 0 01-1 1H3a1 1 0 01-1-1V4a1 1 0 011-1h7.414l2 2zM4 5v14h16V7h-9.414l-2-2H4z" />
        );
      case 'file_code_line':
      case 'file_import_line':
        return (
          <path d="M15 4H5v16h14V8h-4V4zM3 2.992C3 2.444 3.447 2 3.999 2H16l5 5v13.993A1 1 0 0120.007 22H3.993A1 1 0 013 21.008V2.992zM8.5 11.5L6 14l2.5 2.5 1.414-1.414L8.328 14l1.586-1.586L8.5 11.5zm7 0l-1.414 1.414L15.672 14l-1.586 1.586L15.5 16.5l2.5-2.5-2.5-2.5z" />
        );
      case 'eye_line':
        return (
          <path d="M12 3c5.392 0 9.878 3.88 10.819 9-.941 5.12-5.427 9-10.819 9-5.392 0-9.878-3.88-10.819-9C2.121 6.88 6.608 3 12 3zm0 16a7.001 7.001 0 006.743-5c-.777-2.92-3.415-5-6.743-5s-5.966 2.08-6.743 5A7.001 7.001 0 0012 19zm0-3a4 4 0 110-8 4 4 0 010 8zm0-2a2 2 0 100-4 2 2 0 000 4z" />
        );
      case 'eye_close_line':
        return (
          <path d="M4.52 5.934L1.393 2.808l1.415-1.415 19.799 19.8-1.415 1.414-3.31-3.31A10.949 10.949 0 0112 21c-5.392 0-9.878-3.88-10.819-9a10.947 10.947 0 014.52-5.934zM12 19a8.959 8.959 0 005.41-1.815l-2.072-2.071A4 4 0 019.88 9.88L7.752 7.753A8.96 8.96 0 003.18 12C3.958 14.92 6.596 17 12 19zm8.82-7c-.432 1.62-1.42 3.033-2.784 4.093l-1.45-1.45A7.001 7.001 0 0018.743 12c-.777-2.92-3.415-5-6.743-5a6.974 6.974 0 00-3.36.855L7.17 6.386A8.97 8.97 0 0112 5c5.392 0 9.878 3.88 10.819 9z" />
        );
      case 'copy_line':
        return (
          <path d="M7 6V3a1 1 0 011-1h12a1 1 0 011 1v12a1 1 0 01-1 1h-3v3a1 1 0 01-1 1H3a1 1 0 01-1-1V8a1 1 0 011-1h4zm2 2v10h6V8H9zm8-2V4H9v2h8z" />
        );
      case 'delete_2_line':
      case 'delete_fill':
        return (
          <path d="M17 6h5v2h-2v13a1 1 0 01-1 1H5a1 1 0 01-1-1V8H2V6h5V3a1 1 0 011-1h8a1 1 0 011 1v3zm-8 5v6h2v-6H9zm4 0v6h2v-6h-2zM9 4v2h6V4H9z" />
        );
      case 'storage_line':
        return (
          <path d="M4 3h16a1 1 0 011 1v4a1 1 0 01-1 1H4a1 1 0 01-1-1V4a1 1 0 011-1zm0 7h16a1 1 0 011 1v4a1 1 0 01-1 1H4a1 1 0 01-1-1v-4a1 1 0 011-1zm0 7h16a1 1 0 011 1v4a1 1 0 01-1 1H4a1 1 0 01-1-1v-4a1 1 0 011-1zm2-12v2h2V5H6zm0 7v2h2v-2H6zm0 7v2h2v-2H6z" />
        );
      case 'download_line':
        return (
          <path d="M13 10h5l-6 6l-6-6h5V3h2v7zm-9 9h16v2H4v-2z" />
        );
      case 'key_line':
        return (
          <path d="M12.917 11.5a6 6 0 1 0-2.417 2.417l.083.083h2.917v3h3v3h3v-3.5a.5.5 0 0 0-.146-.354l-2.688-2.687a.5.5 0 0 0-.354-.146h-.98zM8 12a4 4 0 1 1 0-8 4 4 0 0 1 0 8z" />
        );
      case 'plus_line':
      case 'add_line':
        return (
          <path d="M11 11V5h2v6h6v2h-6v6h-2v-6H5v-2h6z" />
        );
      case 'edit_line':
        return (
          <path d="M15.728 9.686l-1.414-1.414L5 17.586V19h1.414l9.314-9.314zm1.414-1.414l1.414-1.414a1 1 0 000-1.414l-1.414-1.414a1 1 0 00-1.414 0l-1.414 1.414 2.828 2.828zM3 21v-4.243L16.435 3.322a3 3 0 014.243 0l1.414 1.414a3 3 0 010 4.243L8.657 22.414A1 1 0 017.95 22.707H3.707A.707.707 0 013 22v-1z" />
        );
      case 'arrow_up_line':
      case 'up_line':
        return (
          <path d="M13 7.828V20h-2V7.828l-5.364 5.364-1.414-1.414L12 4l7.778 7.778-1.414 1.414L13 7.828z" />
        );
      case 'arrow_down_line':
      case 'down_line':
        return (
          <path d="M13 16.172l5.364-5.364 1.414 1.414L12 20l-7.778-7.778 1.414-1.414L11 16.172V4h2v12.172z" />
        );
      case 'check_line':
        return (
          <path d="M10 15.172l9.192-9.192 1.414 1.414L10 18l-6.364-6.364 1.414-1.414z" />
        );
      case 'close_line':
        return (
          <path d="M12 10.586l4.95-4.95 1.414 1.414-4.95 4.95 4.95 4.95-1.414 1.414-4.95-4.95-4.95 4.95-1.414-1.414 4.95-4.95-4.95-4.95L7.05 5.636z" />
        );
      case 'chevron_right_line':
        return (
          <path d="M13.172 12l-4.95-4.95 1.414-1.414L16 12l-6.364 6.364-1.414-1.414z" />
        );
      case 'chevron_left_line':
        return (
          <path d="M10.828 12l4.95 4.95-1.414 1.414L8 12l6.364-6.364 1.414 1.414z" />
        );
      case 'expand_line':
        return (
          <path d="M4 4h6v2H6v4H4V4zm10 0h6v6h-2V6h-4V4zM4 14h2v4h4v2H4v-6zm14 0h2v6h-6v-2h4v-4z" />
        );
      case 'chevron_down_line':
        return (
          <path d="M12 13.172l4.95-4.95 1.414 1.414L12 16 5.636 9.636 7.05 8.222z" />
        );
      case 'refresh_line':
      case 'loading_line':
        return (
          <path d="M5.463 4.433A9.961 9.961 0 0 1 12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10A9.96 9.96 0 0 1 2.458 14.5h2.083A7.96 7.96 0 0 0 12 20a8 8 0 1 0 0-16c-2.022 0-3.876.75-5.297 1.993l2.797 2.797H2.5V1.793l2.963 2.64z" />
        );
      case 'terminal_line':
        return (
          <path d="M3 3h18a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zm1 2v14h16V5H4zm3 3l5 4-5 4V8zm7 7h4v2h-4v-2z" />
        );
      case 'lock_line':
        return (
          <path d="M19 10h1a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V11a1 1 0 0 1 1-1h1V7a7 7 0 1 1 14 0v3zm-2 0V7A5 5 0 0 0 7 7v3h10zm-5 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4z" />
        );
      case 'unlock_line':
        return (
          <path d="M7 10h13a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V11a1 1 0 0 1 1-1h3V7a5 5 0 0 1 9.9-1.002l-1.978.33A3 3 0 0 0 9 7v3zm-2 2v8h14v-8H5zm7 2a2 2 0 1 1 0 4 2 2 0 0 1 0-4z" />
        );
      case 'calculator_line':
        return (
          <path d="M4 2h16a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1zm1 2v16h14V4H5zm2 2h10v3H7V6zm0 5h2v2H7v-2zm0 4h2v2H7v-2zm4-4h2v2h-2v-2zm0 4h2v2h-2v-2zm4-4h2v6h-2v-6z" />
        );
      case 'code_line':
        return (
          <path d="M24 12l-5.657 5.657-1.414-1.414L21.172 12l-4.243-4.243 1.414-1.414L24 12zM0 12l5.657-5.657 1.414 1.414L2.828 12l4.243 4.243-1.414 1.414L0 12zm14.544-8.878l1.912.586-6 19.586-1.912-.586 6-19.586z" />
        );
      case 'hash_line':
        return (
          <path d="M7.784 14l.42-4H4V8h4.415l.525-5h2.011l-.525 5h3.989l.525-5h2.011l-.525 5H20v2h-3.989l-.42 4H20v2h-4.415l-.525 5h-2.011l.525-5H9.585l-.525 5H7.049l.525-5H4v-2h3.784zm2.011 0h3.989l.42-4H10.215l-.42 4z" />
        );
      case 'earth_line':
      case 'globe_line':
        return (
          <path d="M12 2a10 10 0 1010 10A10 10 0 0012 2zm-6.93 6h13.86a8.003 8.003 0 00-4.63-4.66A16.035 16.035 0 0012 4c-.79 0-1.57.06-2.3.18A8.003 8.003 0 005.07 8zM12 20c.79 0 1.57-.06 2.3-.18a8.003 8.003 0 004.63-4.66H5.07a8.003 8.003 0 004.63 4.66c.73.12 1.51.18 2.3.18zM4.06 10a8.04 8.04 0 000 4h3.19a18.23 18.23 0 010-4H4.06zm5.22 0a16.2 16.2 0 000 4h5.44a16.2 16.2 0 000-4H9.28zm7.47 0c.06.65.1 1.32.1 2s-.04 1.35-.1 2h3.19a8.04 8.04 0 000-4h-3.19z" />
        );
      case 'alert_line':
      case 'warning_line':
        return (
          <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm1 15h-2v-2h2zm0-4h-2V7h2z" />
        );
      case 'layout_right_line':
        return (
          <path d="M19 3a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h14zm-5 2H5v14h9V5zm5 0h-3v14h3V5z" />
        );
      default:
        // Clean neutral circle fallback instead of warning sign
        return (
          <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8a8 8 0 0 1-8 8zm0-5a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
        );
    }
  };

  return (
    <svg
      width={iconSize}
      height={iconSize}
      viewBox="0 0 24 24"
      fill="currentColor"
      className={`inline-block shrink-0 align-middle transition-colors ${className}`}
      style={style}
    >
      {renderSvgPath()}
    </svg>
  );
};
