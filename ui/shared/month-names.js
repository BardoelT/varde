// The names of the months and weekdays in the chosen language (CSD-022). Display only.
import { t, currentLanguage } from './i18n.js';

const FULL = () => [t('January'), t('February'), t('March'), t('April'), t('May'), t('June'), t('July'), t('August'), t('September'), t('October'), t('November'), t('December')];
const SHORT = () => [t('Jan'), t('Feb'), t('Mar'), t('Apr'), t('May'), t('Jun'), t('Jul'), t('Aug'), t('Sep'), t('Oct'), t('Nov'), t('Dec')];
const WEEKDAYS = () => [t('Mon'), t('Tue'), t('Wed'), t('Thu'), t('Fri'), t('Sat'), t('Sun')];

/**
 * The full name of a month, `index` 0 (January) to 11. Dutch writes month names in lower case ("november 2026"),
 * and the key "May" is shared with the short name, which is capitalised, so the name is lower-cased here.
 */
export function monthName(index) {
  const name = FULL()[index];
  return currentLanguage() === 'en' ? name : name.toLocaleLowerCase(currentLanguage());
}

/** The short name of a month, `index` 0 to 11. */
export const shortMonthName = (index) => SHORT()[index];

/** The short weekday names, Monday first. */
export const weekdayNames = () => WEEKDAYS();
